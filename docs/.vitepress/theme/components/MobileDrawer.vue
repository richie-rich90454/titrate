<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="drawer-backdrop"
      @click="$emit('close')"
    />
    <aside
      class="mobile-drawer"
      :class="{ open }"
      role="dialog"
      aria-label="Navigation"
    >
      <div class="drawer-header">
        <span class="drawer-title">Navigation</span>
        <button class="drawer-close" @click="$emit('close')" aria-label="Close menu">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M6 18L18 6M6 6l12 12" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      </div>
      <nav class="drawer-nav" aria-label="Sidebar navigation">
        <slot />
      </nav>
    </aside>
  </Teleport>
</template>

<script setup>
defineProps({
  open: Boolean
})

defineEmits(['close'])
</script>

<style scoped>
.drawer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 90;
  background: rgba(0, 0, 0, 0.4);
  animation: fadeIn 150ms ease;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.mobile-drawer {
  position: fixed;
  top: 0;
  left: 0;
  bottom: 0;
  z-index: 100;
  width: 280px;
  max-width: 85vw;
  background: var(--tt-bg);
  border-right: 1px solid var(--tt-border);
  transform: translateX(-100%);
  transition: transform 200ms cubic-bezier(0.4, 0, 0.2, 1);
  overflow-y: auto;
  overscroll-behavior: contain;
}

.mobile-drawer.open {
  transform: translateX(0);
}

.drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  border-bottom: 1px solid var(--tt-border);
}

.drawer-title {
  font-weight: 600;
  font-size: 0.95em;
}

.drawer-close {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--tt-text-2);
  cursor: pointer;
  border-radius: var(--tt-radius-sm);
  transition: all var(--tt-transition);
}

.drawer-close:hover {
  background: var(--tt-hover);
  color: var(--tt-text);
}

.drawer-nav {
  padding: 8px 0;
}

@media (min-width: 769px) {
  .drawer-backdrop,
  .mobile-drawer {
    display: none;
  }
}
</style>
