//! Host-side configuration and validation for the kernel substrate.
//!
//! Phase 14 of `ROADMAP.md` calls for a freestanding kernel: IDT and interrupt
//! handling, a scheduler, page tables, drivers, and a boot protocol. This
//! module is the inert, host-side half of that work. It describes and validates
//! a kernel image before any freestanding code is emitted; it never calls into
//! the compiler, linker, CLI, or hosted runtime, so hosted behavior is
//! unchanged. The runtime half of the design lives in
//! `docs/plans/kernel-substrate.md`.

use std::error::Error;
use std::fmt;

/// Page size used for every region alignment check (4 KiB).
pub const PAGE_SIZE: u64 = 4096;

/// Number of entries in the x86-64 interrupt descriptor table.
pub const IDT_ENTRIES: u16 = 256;

/// Vectors `0..CPU_EXCEPTION_VECTOR_COUNT` are reserved by the CPU for
/// exceptions and faults. The allocator only hands out vectors at or above
/// this value.
pub const CPU_EXCEPTION_VECTOR_COUNT: u16 = 32;

/// Highest number of scheduler priority levels the model accepts.
pub const MAX_PRIORITY_LEVELS: u16 = 256;

/// CPU features the kernel may depend on. These are guarantees the boot
/// environment must provide, not probes performed here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuFeature {
    /// Local APIC for interrupt delivery.
    Apic,
    /// x2APIC extended register interface (implies [`CpuFeature::Apic`]).
    X2Apic,
    /// SSE2 register state, baseline on x86-64.
    Sse2,
    /// XSAVE/XRSTOR for extended register state.
    XSave,
    /// AVX register state (implies [`CpuFeature::XSave`]).
    Avx,
    /// 2 MiB pages via the PDE page-size bit.
    Page2M,
    /// 1 GiB pages via the PDPTE page-size bit.
    Page1G,
}

impl CpuFeature {
    /// Stable name used in diagnostics and configuration files.
    pub fn as_str(self) -> &'static str {
        match self {
            CpuFeature::Apic => "apic",
            CpuFeature::X2Apic => "x2apic",
            CpuFeature::Sse2 => "sse2",
            CpuFeature::XSave => "xsave",
            CpuFeature::Avx => "avx",
            CpuFeature::Page2M => "page2m",
            CpuFeature::Page1G => "page1g",
        }
    }

    /// Bit position in the [`CpuFeatures`] mask.
    const fn bit(self) -> u128 {
        1u128 << (self as u128)
    }

    /// Features that must also be required when this one is required.
    fn prerequisites(self) -> &'static [CpuFeature] {
        match self {
            CpuFeature::X2Apic => &[CpuFeature::Apic],
            CpuFeature::Avx => &[CpuFeature::XSave],
            _ => &[],
        }
    }
}

/// Every known feature, used to walk the set during validation.
const ALL_FEATURES: &[CpuFeature] = &[
    CpuFeature::Apic,
    CpuFeature::X2Apic,
    CpuFeature::Sse2,
    CpuFeature::XSave,
    CpuFeature::Avx,
    CpuFeature::Page2M,
    CpuFeature::Page1G,
];

/// A set of required CPU features, stored as a bit mask.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CpuFeatures(u128);

impl CpuFeatures {
    /// No features required.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Add `feature` to the required set.
    pub fn require(&mut self, feature: CpuFeature) {
        self.0 |= feature.bit();
    }

    /// Whether `feature` is in the required set.
    pub fn contains(self, feature: CpuFeature) -> bool {
        self.0 & feature.bit() != 0
    }

    /// Number of required features.
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Check that every prerequisite of every required feature is present.
    fn validate(self) -> Result<(), KernelConfigError> {
        for &feature in ALL_FEATURES {
            if !self.contains(feature) {
                continue;
            }
            for &needed in feature.prerequisites() {
                if !self.contains(needed) {
                    return Err(KernelConfigError::new(
                        "cpu_features",
                        format!(
                            "required feature '{}' needs '{}'",
                            feature.as_str(),
                            needed.as_str()
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Purpose of a physical memory region in the boot image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionKind {
    /// Executable kernel code. Mapped read-execute.
    Text,
    /// Read-only kernel data. Mapped read-only.
    Rodata,
    /// Writable kernel data. Mapped read-write.
    Data,
    /// Frame allocator backing storage and kernel heap.
    Heap,
    /// Memory-mapped device registers. Mapped uncached.
    Mmio,
}

impl RegionKind {
    /// Stable name used in diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            RegionKind::Text => "text",
            RegionKind::Rodata => "rodata",
            RegionKind::Data => "data",
            RegionKind::Heap => "heap",
            RegionKind::Mmio => "mmio",
        }
    }
}

/// A half-open physical address range `[start, end)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryRegion {
    kind: RegionKind,
    start: u64,
    end: u64,
}

impl MemoryRegion {
    /// Create a region. Alignment, ordering, and overlap are checked by
    /// [`MemoryLayout::validate`] so every diagnostic carries one field tag.
    pub fn new(kind: RegionKind, start: u64, end: u64) -> Self {
        Self { kind, start, end }
    }

    /// Purpose of this region.
    pub fn kind(&self) -> RegionKind {
        self.kind
    }

    /// Inclusive start address.
    pub fn start(&self) -> u64 {
        self.start
    }

    /// Exclusive end address.
    pub fn end(&self) -> u64 {
        self.end
    }

    /// Length in bytes. Saturates on an unvalidated, inverted range.
    pub fn len(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    /// Whether the range is empty. Saturating keeps this safe before validation.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Physical memory layout of the boot image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryLayout {
    regions: Vec<MemoryRegion>,
}

impl MemoryLayout {
    /// Wrap a list of regions; validation is deferred to the builder.
    pub fn new(regions: Vec<MemoryRegion>) -> Self {
        Self { regions }
    }

    /// All regions, in declaration order.
    pub fn regions(&self) -> &[MemoryRegion] {
        &self.regions
    }

    /// First region containing `address`, if any.
    pub fn region_at(&self, address: u64) -> Option<&MemoryRegion> {
        self.regions
            .iter()
            .find(|region| address >= region.start && address < region.end)
    }

    /// Check alignment, ordering, non-overlap, and the presence of a text
    /// region. Regions may be declared in any order.
    fn validate(&self) -> Result<(), KernelConfigError> {
        if self.regions.is_empty() {
            return Err(KernelConfigError::new(
                "memory_layout",
                "at least one region is required",
            ));
        }

        let mut sorted = self.regions.clone();
        sorted.sort_by_key(|region| (region.start, region.end));

        let mut previous_end = 0u64;
        for (index, region) in sorted.iter().enumerate() {
            if region.start >= region.end {
                return Err(KernelConfigError::new(
                    "memory_layout",
                    format!(
                        "{} region must have start < end (got {:#x}..{:#x})",
                        region.kind.as_str(),
                        region.start,
                        region.end
                    ),
                ));
            }
            if region.start % PAGE_SIZE != 0 || region.end % PAGE_SIZE != 0 {
                return Err(KernelConfigError::new(
                    "memory_layout",
                    format!(
                        "{} region bounds must be {} KiB aligned (got {:#x}..{:#x})",
                        region.kind.as_str(),
                        PAGE_SIZE / 1024,
                        region.start,
                        region.end
                    ),
                ));
            }
            if index > 0 && region.start < previous_end {
                return Err(KernelConfigError::new(
                    "memory_layout",
                    format!(
                        "{} region at {:#x} overlaps the previous region ending at {:#x}",
                        region.kind.as_str(),
                        region.start,
                        previous_end
                    ),
                ));
            }
            previous_end = region.end;
        }

        if !sorted
            .iter()
            .any(|region| region.kind == RegionKind::Text)
        {
            return Err(KernelConfigError::new(
                "memory_layout",
                "a text region is required for the kernel image",
            ));
        }
        Ok(())
    }
}

/// A request to reserve contiguous IDT vectors for one interrupt source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterruptRequest {
    name: String,
    count: u16,
    preferred: Option<u16>,
}

impl InterruptRequest {
    /// Request `count` contiguous vectors for a uniquely named source.
    pub fn new(name: impl Into<String>, count: u16) -> Self {
        Self {
            name: name.into(),
            count,
            preferred: None,
        }
    }

    /// Pin the first vector. Without this, the lowest free run is chosen.
    pub fn at(mut self, vector: u16) -> Self {
        self.preferred = Some(vector);
        self
    }

    /// Source name, used for diagnostics and lookup.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Number of contiguous vectors requested.
    pub fn count(&self) -> u16 {
        self.count
    }

    /// Pinned first vector, if any.
    pub fn preferred(&self) -> Option<u16> {
        self.preferred
    }
}

/// A resolved vector allocation for one interrupt source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterruptAllocation {
    name: String,
    first: u16,
    /// Exclusive end vector.
    end: u16,
}

impl InterruptAllocation {
    /// Source name this allocation belongs to.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// First allocated vector, inclusive.
    pub fn first(&self) -> u16 {
        self.first
    }

    /// One past the last allocated vector, exclusive.
    pub fn end(&self) -> u16 {
        self.end
    }

    /// Whether `vector` belongs to this allocation.
    pub fn contains(&self, vector: u16) -> bool {
        vector >= self.first && vector < self.end
    }
}

/// The allocated IDT vector map. CPU exception vectors are never handed out.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterruptVectorTable {
    allocations: Vec<InterruptAllocation>,
}

impl InterruptVectorTable {
    /// All allocations, in request order.
    pub fn allocations(&self) -> &[InterruptAllocation] {
        &self.allocations
    }

    /// Look up the allocation for a named source.
    pub fn lookup(&self, name: &str) -> Option<&InterruptAllocation> {
        self.allocations
            .iter()
            .find(|allocation| allocation.name.as_str() == name)
    }

    /// Allocate vectors for `requests` in declaration order. A pinned request
    /// must fit exactly where asked; an unpinned request takes the lowest free
    /// run. Allocation is deterministic and fails instead of silently moving a
    /// request that cannot be honored.
    fn allocate(requests: &[InterruptRequest]) -> Result<Self, KernelConfigError> {
        let mut used = [false; IDT_ENTRIES as usize];
        for vector in 0..CPU_EXCEPTION_VECTOR_COUNT {
            used[vector as usize] = true;
        }

        let mut names: Vec<&str> = Vec::with_capacity(requests.len());
        let mut allocations = Vec::with_capacity(requests.len());

        for request in requests {
            if request.name.is_empty() {
                return Err(KernelConfigError::new(
                    "interrupt_vectors",
                    "interrupt source name must not be empty",
                ));
            }
            if names.contains(&request.name.as_str()) {
                return Err(KernelConfigError::new(
                    "interrupt_vectors",
                    format!("duplicate interrupt source '{}'", request.name),
                ));
            }
            names.push(request.name.as_str());

            if request.count == 0 {
                return Err(KernelConfigError::new(
                    "interrupt_vectors",
                    format!("'{}' must request at least one vector", request.name),
                ));
            }
            if request.count > IDT_ENTRIES - CPU_EXCEPTION_VECTOR_COUNT {
                return Err(KernelConfigError::new(
                    "interrupt_vectors",
                    format!(
                        "'{}' requests {} vectors; only {} are allocatable",
                        request.name,
                        request.count,
                        IDT_ENTRIES - CPU_EXCEPTION_VECTOR_COUNT
                    ),
                ));
            }

            let first = match request.preferred {
                Some(vector) => {
                    let end = vector.checked_add(request.count).ok_or_else(|| {
                        KernelConfigError::new(
                            "interrupt_vectors",
                            format!("'{}' vector range overflows", request.name),
                        )
                    })?;
                    if vector < CPU_EXCEPTION_VECTOR_COUNT {
                        return Err(KernelConfigError::new(
                            "interrupt_vectors",
                            format!(
                                "'{}' cannot use reserved CPU exception vector {}",
                                request.name, vector
                            ),
                        ));
                    }
                    if end > IDT_ENTRIES {
                        return Err(KernelConfigError::new(
                            "interrupt_vectors",
                            format!(
                                "'{}' vector range {}..{} exceeds the IDT",
                                request.name, vector, end
                            ),
                        ));
                    }
                    if used[vector as usize..end as usize].iter().any(|&taken| taken) {
                        return Err(KernelConfigError::new(
                            "interrupt_vectors",
                            format!(
                                "'{}' preferred vector {} is unavailable",
                                request.name, vector
                            ),
                        ));
                    }
                    vector
                }
                None => {
                    let last_first = IDT_ENTRIES - request.count;
                    let mut candidate = CPU_EXCEPTION_VECTOR_COUNT;
                    let mut found = None;
                    while candidate <= last_first {
                        let end = candidate + request.count;
                        if !used[candidate as usize..end as usize]
                            .iter()
                            .any(|&taken| taken)
                        {
                            found = Some(candidate);
                            break;
                        }
                        candidate += 1;
                    }
                    found.ok_or_else(|| {
                        KernelConfigError::new(
                            "interrupt_vectors",
                            format!(
                                "no free range of {} vectors for '{}'",
                                request.count, request.name
                            ),
                        )
                    })?
                }
            };

            let end = first + request.count;
            for vector in first..end {
                used[vector as usize] = true;
            }
            allocations.push(InterruptAllocation {
                name: request.name.clone(),
                first,
                end,
            });
        }

        Ok(Self { allocations })
    }
}

/// Kernel thread scheduling policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerPolicy {
    /// All runnable threads share one queue and rotate on a fixed time slice.
    RoundRobin,
    /// Round robin within each priority level; lower-numbered levels run first.
    PriorityRoundRobin,
    /// Earliest-deadline-first among runnable threads.
    EarliestDeadlineFirst,
}

impl SchedulerPolicy {
    /// Stable name used in diagnostics and configuration files.
    pub fn as_str(self) -> &'static str {
        match self {
            SchedulerPolicy::RoundRobin => "round-robin",
            SchedulerPolicy::PriorityRoundRobin => "priority-round-robin",
            SchedulerPolicy::EarliestDeadlineFirst => "earliest-deadline-first",
        }
    }

    /// Whether this policy consults the configured priority levels.
    fn uses_priorities(self) -> bool {
        match self {
            SchedulerPolicy::RoundRobin => false,
            SchedulerPolicy::PriorityRoundRobin | SchedulerPolicy::EarliestDeadlineFirst => true,
        }
    }
}

/// Scheduler configuration carried by a validated kernel substrate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedulerConfig {
    policy: SchedulerPolicy,
    time_slice_ticks: u64,
    priority_levels: u16,
    max_threads: u32,
}

impl SchedulerConfig {
    /// Create a scheduler configuration. All numeric fields are validated by
    /// [`SchedulerConfig::validate`] during the build.
    pub fn new(
        policy: SchedulerPolicy,
        time_slice_ticks: u64,
        priority_levels: u16,
        max_threads: u32,
    ) -> Self {
        Self {
            policy,
            time_slice_ticks,
            priority_levels,
            max_threads,
        }
    }

    /// Scheduling policy.
    pub fn policy(&self) -> SchedulerPolicy {
        self.policy
    }

    /// Number of timer ticks per time slice.
    pub fn time_slice_ticks(&self) -> u64 {
        self.time_slice_ticks
    }

    /// Number of priority levels.
    pub fn priority_levels(&self) -> u16 {
        self.priority_levels
    }

    /// Upper bound on simultaneously runnable threads.
    pub fn max_threads(&self) -> u32 {
        self.max_threads
    }

    fn validate(&self) -> Result<(), KernelConfigError> {
        if self.time_slice_ticks == 0 {
            return Err(KernelConfigError::new(
                "scheduler",
                "time_slice_ticks must be greater than zero",
            ));
        }
        if self.max_threads == 0 {
            return Err(KernelConfigError::new(
                "scheduler",
                "max_threads must be greater than zero",
            ));
        }
        if self.priority_levels == 0 {
            return Err(KernelConfigError::new(
                "scheduler",
                "priority_levels must be greater than zero",
            ));
        }
        if self.priority_levels > MAX_PRIORITY_LEVELS {
            return Err(KernelConfigError::new(
                "scheduler",
                format!(
                    "priority_levels {} exceeds the maximum of {}",
                    self.priority_levels, MAX_PRIORITY_LEVELS
                ),
            ));
        }
        if !self.policy.uses_priorities() && self.priority_levels != 1 {
            return Err(KernelConfigError::new(
                "scheduler",
                format!(
                    "policy '{}' does not use priority levels; set priority_levels to 1",
                    self.policy.as_str()
                ),
            ));
        }
        Ok(())
    }
}

/// Fully validated kernel substrate configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelSubstrateConfig {
    cpu: CpuFeatures,
    memory: MemoryLayout,
    interrupts: InterruptVectorTable,
    scheduler: SchedulerConfig,
}

impl KernelSubstrateConfig {
    /// Start a new configuration builder.
    pub fn builder() -> KernelSubstrateBuilder {
        KernelSubstrateBuilder::new()
    }

    /// Required CPU features.
    pub fn cpu_features(&self) -> CpuFeatures {
        self.cpu
    }

    /// Validated physical memory layout.
    pub fn memory(&self) -> &MemoryLayout {
        &self.memory
    }

    /// Allocated interrupt vector map.
    pub fn interrupts(&self) -> &InterruptVectorTable {
        &self.interrupts
    }

    /// Scheduler configuration.
    pub fn scheduler(&self) -> &SchedulerConfig {
        &self.scheduler
    }
}

/// A validation failure, tagged with the field that caused it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelConfigError {
    field: &'static str,
    message: String,
}

impl KernelConfigError {
    fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }

    /// Name of the invalid field, suitable for a diagnostic.
    pub fn field(&self) -> &'static str {
        self.field
    }

    /// Human-readable explanation of the failure.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for KernelConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl Error for KernelConfigError {}

/// Incremental builder for [`KernelSubstrateConfig`].
///
/// The memory layout and scheduler are required. CPU features and interrupt
/// requests may be empty: a kernel with no optional features and no devices
/// beyond the timer is a valid minimal substrate.
#[derive(Clone, Debug, Default)]
pub struct KernelSubstrateBuilder {
    cpu: CpuFeatures,
    regions: Vec<MemoryRegion>,
    interrupts: Vec<InterruptRequest>,
    scheduler: Option<SchedulerConfig>,
}

impl KernelSubstrateBuilder {
    /// Create an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Require a CPU feature in the boot environment.
    pub fn require_feature(mut self, feature: CpuFeature) -> Self {
        self.cpu.require(feature);
        self
    }

    /// Add a physical memory region to the layout.
    pub fn region(mut self, region: MemoryRegion) -> Self {
        self.regions.push(region);
        self
    }

    /// Add an interrupt vector request.
    pub fn interrupt(mut self, request: InterruptRequest) -> Self {
        self.interrupts.push(request);
        self
    }

    /// Set the scheduler configuration.
    pub fn scheduler(mut self, config: SchedulerConfig) -> Self {
        self.scheduler = Some(config);
        self
    }

    /// Validate every field and produce an immutable configuration.
    pub fn build(self) -> Result<KernelSubstrateConfig, KernelConfigError> {
        self.cpu.validate()?;

        let memory = MemoryLayout::new(self.regions);
        memory.validate()?;

        let interrupts = InterruptVectorTable::allocate(&self.interrupts)?;

        let scheduler = self.scheduler.ok_or_else(|| {
            KernelConfigError::new("scheduler", "required for a kernel substrate")
        })?;
        scheduler.validate()?;

        Ok(KernelSubstrateConfig {
            cpu: self.cpu,
            memory,
            interrupts,
            scheduler,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> KernelSubstrateBuilder {
        KernelSubstrateBuilder::new()
            .region(MemoryRegion::new(RegionKind::Text, 0x10_0000, 0x20_0000))
            .scheduler(SchedulerConfig::new(
                SchedulerPolicy::RoundRobin,
                10,
                1,
                32,
            ))
    }

    // CPU features

    #[test]
    fn empty_features_validate() {
        assert_eq!(CpuFeatures::empty(), CpuFeatures::default());
        assert_eq!(CpuFeatures::empty().count(), 0);
        minimal().build().expect("no required features is valid");
    }

    #[test]
    fn features_are_inserted_and_queried() {
        let mut features = CpuFeatures::empty();
        features.require(CpuFeature::Apic);
        features.require(CpuFeature::Sse2);
        features.require(CpuFeature::Apic);
        assert!(features.contains(CpuFeature::Apic));
        assert!(features.contains(CpuFeature::Sse2));
        assert!(!features.contains(CpuFeature::Avx));
        assert_eq!(features.count(), 2);
    }

    #[test]
    fn satisfied_prerequisites_build() {
        let config = minimal()
            .require_feature(CpuFeature::Apic)
            .require_feature(CpuFeature::X2Apic)
            .require_feature(CpuFeature::XSave)
            .require_feature(CpuFeature::Avx)
            .build()
            .expect("all prerequisites present");
        assert_eq!(config.cpu_features().count(), 4);
    }

    #[test]
    fn missing_prerequisites_are_rejected() {
        let cases = [
            (CpuFeature::X2Apic, "apic"),
            (CpuFeature::Avx, "xsave"),
        ];
        for (feature, needed) in cases {
            let err = minimal()
                .require_feature(feature)
                .build()
                .unwrap_err();
            assert_eq!(err.field(), "cpu_features", "feature: {}", feature.as_str());
            assert!(err.message().contains(needed));
        }
    }

    #[test]
    fn feature_names_are_stable() {
        assert_eq!(CpuFeature::Apic.as_str(), "apic");
        assert_eq!(CpuFeature::X2Apic.as_str(), "x2apic");
        assert_eq!(CpuFeature::Sse2.as_str(), "sse2");
        assert_eq!(CpuFeature::XSave.as_str(), "xsave");
        assert_eq!(CpuFeature::Avx.as_str(), "avx");
        assert_eq!(CpuFeature::Page2M.as_str(), "page2m");
        assert_eq!(CpuFeature::Page1G.as_str(), "page1g");
    }

    // Memory layout

    #[test]
    fn valid_layout_builds_and_looks_up_addresses() {
        let config = minimal()
            .region(MemoryRegion::new(RegionKind::Rodata, 0x20_0000, 0x21_0000))
            .region(MemoryRegion::new(RegionKind::Data, 0x21_0000, 0x22_0000))
            .region(MemoryRegion::new(RegionKind::Heap, 0x22_0000, 0x30_0000))
            .build()
            .expect("contiguous aligned regions are valid");
        let layout = config.memory();
        assert_eq!(layout.regions().len(), 4);
        assert_eq!(
            layout.region_at(0x20_0fff).map(MemoryRegion::kind),
            Some(RegionKind::Rodata)
        );
        assert_eq!(layout.region_at(0x40_0000), None);
    }

    #[test]
    fn regions_may_be_declared_out_of_order() {
        minimal()
            .region(MemoryRegion::new(RegionKind::Data, 0x21_0000, 0x22_0000))
            .region(MemoryRegion::new(RegionKind::Rodata, 0x20_0000, 0x21_0000))
            .build()
            .expect("declaration order must not matter");
    }

    #[test]
    fn empty_layout_is_rejected() {
        let err = KernelSubstrateBuilder::new()
            .scheduler(SchedulerConfig::new(
                SchedulerPolicy::RoundRobin,
                10,
                1,
                32,
            ))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "memory_layout");
    }

    #[test]
    fn inverted_and_empty_regions_are_rejected() {
        for (start, end) in [(0x20_0000, 0x10_0000), (0x10_0000, 0x10_0000)] {
            let err = KernelSubstrateBuilder::new()
                .region(MemoryRegion::new(RegionKind::Text, start, end))
                .scheduler(SchedulerConfig::new(
                    SchedulerPolicy::RoundRobin,
                    10,
                    1,
                    32,
                ))
                .build()
                .unwrap_err();
            assert_eq!(err.field(), "memory_layout");
        }
    }

    #[test]
    fn misaligned_regions_are_rejected() {
        let err = minimal()
            .region(MemoryRegion::new(RegionKind::Heap, 0x20_0001, 0x21_0000))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "memory_layout");
        assert!(err.message().contains("aligned"));
    }

    #[test]
    fn overlapping_regions_are_rejected() {
        let err = minimal()
            .region(MemoryRegion::new(RegionKind::Data, 0x1f_0000, 0x21_0000))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "memory_layout");
        assert!(err.message().contains("overlap"));
    }

    #[test]
    fn layout_without_text_is_rejected() {
        let err = KernelSubstrateBuilder::new()
            .region(MemoryRegion::new(RegionKind::Data, 0x10_0000, 0x20_0000))
            .scheduler(SchedulerConfig::new(
                SchedulerPolicy::RoundRobin,
                10,
                1,
                32,
            ))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "memory_layout");
        assert!(err.message().contains("text"));
    }

    #[test]
    fn region_length_is_saturating() {
        let region = MemoryRegion::new(RegionKind::Text, 0x2000, 0x1000);
        assert_eq!(region.len(), 0);
        assert!(region.is_empty());
        let valid = MemoryRegion::new(RegionKind::Text, 0x1000, 0x3000);
        assert_eq!(valid.len(), 0x2000);
        assert!(!valid.is_empty());
    }

    // Interrupt vectors

    #[test]
    fn no_requests_produces_an_empty_table() {
        let config = minimal().build().expect("minimal config must build");
        assert!(config.interrupts().allocations().is_empty());
        assert!(config.interrupts().lookup("timer").is_none());
    }

    #[test]
    fn vector_ranges_are_allocated_from_the_lowest_free_slot() {
        let config = minimal()
            .interrupt(InterruptRequest::new("timer", 1))
            .interrupt(InterruptRequest::new("keyboard", 2))
            .build()
            .expect("allocation must succeed");
        let table = config.interrupts();
        let timer = table.lookup("timer").expect("timer allocated");
        let keyboard = table.lookup("keyboard").expect("keyboard allocated");
        assert_eq!((timer.first(), timer.end()), (32, 33));
        assert_eq!((keyboard.first(), keyboard.end()), (33, 35));
        assert!(timer.contains(32));
        assert!(!timer.contains(33));
        assert!(keyboard.contains(34));
        assert!(!keyboard.contains(35));
    }

    #[test]
    fn preferred_vector_is_honored() {
        let config = minimal()
            .interrupt(InterruptRequest::new("serial", 4).at(0x30))
            .build()
            .expect("preferred range is free");
        let serial = config
            .interrupts()
            .lookup("serial")
            .expect("serial allocated");
        assert_eq!((serial.first(), serial.end()), (0x30, 0x34));
    }

    #[test]
    fn reserved_cpu_exception_vectors_are_rejected() {
        let err = minimal()
            .interrupt(InterruptRequest::new("timer", 1).at(CPU_EXCEPTION_VECTOR_COUNT - 1))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "interrupt_vectors");
        assert!(err.message().contains("reserved"));
    }

    #[test]
    fn out_of_range_and_overflowing_preferences_are_rejected() {
        let cases = [
            InterruptRequest::new("timer", 1).at(IDT_ENTRIES),
            InterruptRequest::new("timer", 2).at(IDT_ENTRIES - 1),
            InterruptRequest::new("timer", 1).at(u16::MAX),
        ];
        for request in cases {
            let err = minimal().interrupt(request).build().unwrap_err();
            assert_eq!(err.field(), "interrupt_vectors");
        }
    }

    #[test]
    fn overlapping_preferred_ranges_are_rejected() {
        let err = minimal()
            .interrupt(InterruptRequest::new("timer", 4).at(0x30))
            .interrupt(InterruptRequest::new("keyboard", 2).at(0x32))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "interrupt_vectors");
        assert!(err.message().contains("unavailable"));
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let err = minimal()
            .interrupt(InterruptRequest::new("timer", 1))
            .interrupt(InterruptRequest::new("timer", 1))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "interrupt_vectors");
        assert!(err.message().contains("duplicate"));
    }

    #[test]
    fn empty_names_and_zero_counts_are_rejected() {
        let empty = minimal()
            .interrupt(InterruptRequest::new("", 1))
            .build()
            .unwrap_err();
        assert_eq!(empty.field(), "interrupt_vectors");

        let zero = minimal()
            .interrupt(InterruptRequest::new("timer", 0))
            .build()
            .unwrap_err();
        assert_eq!(zero.field(), "interrupt_vectors");
    }

    #[test]
    fn oversized_requests_are_rejected() {
        let err = minimal()
            .interrupt(InterruptRequest::new(
                "timer",
                IDT_ENTRIES - CPU_EXCEPTION_VECTOR_COUNT + 1,
            ))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "interrupt_vectors");
    }

    #[test]
    fn vector_exhaustion_is_rejected() {
        let mut builder = minimal();
        for index in 0..(IDT_ENTRIES - CPU_EXCEPTION_VECTOR_COUNT) {
            builder = builder.interrupt(InterruptRequest::new(format!("dev{}", index), 1));
        }
        let err = builder
            .interrupt(InterruptRequest::new("overflow", 1))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "interrupt_vectors");
        assert!(err.message().contains("no free range"));
    }

    #[test]
    fn request_accessors_round_trip() {
        let request = InterruptRequest::new("serial", 2).at(0x40);
        assert_eq!(request.name(), "serial");
        assert_eq!(request.count(), 2);
        assert_eq!(request.preferred(), Some(0x40));
        assert_eq!(
            InterruptRequest::new("timer", 1).preferred(),
            None
        );
    }

    // Scheduler

    #[test]
    fn priority_policies_accept_multiple_levels() {
        let config = minimal()
            .scheduler(SchedulerConfig::new(
                SchedulerPolicy::PriorityRoundRobin,
                5,
                8,
                64,
            ))
            .build()
            .expect("priority policy is valid");
        let scheduler = config.scheduler();
        assert_eq!(scheduler.policy(), SchedulerPolicy::PriorityRoundRobin);
        assert_eq!(scheduler.time_slice_ticks(), 5);
        assert_eq!(scheduler.priority_levels(), 8);
        assert_eq!(scheduler.max_threads(), 64);
    }

    #[test]
    fn earliest_deadline_first_accepts_priority_levels() {
        minimal()
            .scheduler(SchedulerConfig::new(
                SchedulerPolicy::EarliestDeadlineFirst,
                1,
                2,
                1,
            ))
            .build()
            .expect("EDF policy is valid");
    }

    #[test]
    fn zero_time_slice_is_rejected() {
        let err = minimal()
            .scheduler(SchedulerConfig::new(
                SchedulerPolicy::RoundRobin,
                0,
                1,
                32,
            ))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "scheduler");
        assert!(err.message().contains("time_slice_ticks"));
    }

    #[test]
    fn zero_thread_or_priority_limits_are_rejected() {
        for (levels, threads) in [(1, 0), (0, 32)] {
            let err = minimal()
                .scheduler(SchedulerConfig::new(
                    SchedulerPolicy::RoundRobin,
                    10,
                    levels,
                    threads,
                ))
                .build()
                .unwrap_err();
            assert_eq!(err.field(), "scheduler");
        }
    }

    #[test]
    fn excessive_priority_levels_are_rejected() {
        let err = minimal()
            .scheduler(SchedulerConfig::new(
                SchedulerPolicy::PriorityRoundRobin,
                10,
                MAX_PRIORITY_LEVELS + 1,
                32,
            ))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "scheduler");
    }

    #[test]
    fn round_robin_rejects_extra_priority_levels() {
        let err = minimal()
            .scheduler(SchedulerConfig::new(
                SchedulerPolicy::RoundRobin,
                10,
                2,
                32,
            ))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "scheduler");
        assert!(err.message().contains("priority_levels"));
    }

    #[test]
    fn scheduler_policy_names_are_stable() {
        assert_eq!(SchedulerPolicy::RoundRobin.as_str(), "round-robin");
        assert_eq!(
            SchedulerPolicy::PriorityRoundRobin.as_str(),
            "priority-round-robin"
        );
        assert_eq!(
            SchedulerPolicy::EarliestDeadlineFirst.as_str(),
            "earliest-deadline-first"
        );
    }

    // Builder and errors

    #[test]
    fn scheduler_is_required() {
        let err = KernelSubstrateBuilder::new()
            .region(MemoryRegion::new(RegionKind::Text, 0x10_0000, 0x20_0000))
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "scheduler");
    }

    #[test]
    fn builder_entry_point_matches_new() {
        let config = KernelSubstrateConfig::builder()
            .region(MemoryRegion::new(RegionKind::Text, 0x10_0000, 0x20_0000))
            .scheduler(SchedulerConfig::new(
                SchedulerPolicy::RoundRobin,
                10,
                1,
                32,
            ))
            .build()
            .expect("config must build");
        assert_eq!(config.memory().regions().len(), 1);
    }

    #[test]
    fn config_error_displays_field_and_message() {
        let err = KernelSubstrateBuilder::new().build().unwrap_err();
        assert!(err.to_string().starts_with("memory_layout: "));
        assert_eq!(err.field(), "memory_layout");
        assert!(!err.message().is_empty());
    }

    #[test]
    fn validation_precedence_is_stable() {
        let err = KernelSubstrateBuilder::new()
            .require_feature(CpuFeature::X2Apic)
            .build()
            .unwrap_err();
        assert_eq!(err.field(), "cpu_features");
    }
}
