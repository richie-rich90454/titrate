// Phase 12 helper natives for mesh, lighting, MIDI, and filter operations.
//
// These are additive host primitives. Registration is exposed as a helper
// function so callers can extend the native lookup without editing mod.rs or
// lookup.rs; no existing dispatch entry is changed.

use super::super::super::value::{NativeFn, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const MAX_VERTICES: usize = 1 << 20;
const MAX_FACES: usize = 1 << 20;

struct ExtMesh {
    vertices: Vec<[f64; 3]>,
    faces: Vec<[usize; 3]>,
}

thread_local! {
    static MESHES: RefCell<HashMap<i64, ExtMesh>> = RefCell::new(HashMap::new());
    static NEXT_MESH: Cell<i64> = const { Cell::new(1) };
}

fn int_arg(args: &[Value], index: usize) -> i64 {
    match args.get(index) {
        Some(Value::Int(v)) => *v as i64,
        Some(Value::Long(v)) => *v,
        _ => 0,
    }
}

fn f64_arg(args: &[Value], index: usize) -> f64 {
    args.get(index).and_then(Value::to_f64).unwrap_or(0.0)
}

pub(crate) fn native_ext_mesh_new(_args: &[Value]) -> Result<Value, String> {
    let handle = NEXT_MESH.with(|next| {
        let id = next.get();
        next.set(id.saturating_add(1));
        id
    });
    MESHES.with(|meshes| {
        meshes.borrow_mut().insert(
            handle,
            ExtMesh {
                vertices: Vec::new(),
                faces: Vec::new(),
            },
        );
    });
    Ok(Value::Long(handle))
}

pub(crate) fn native_ext_mesh_add_vertex(args: &[Value]) -> Result<Value, String> {
    let vertex = [f64_arg(args, 1), f64_arg(args, 2), f64_arg(args, 3)];
    let added = MESHES.with(|meshes| {
        let mut meshes = meshes.borrow_mut();
        match meshes.get_mut(&int_arg(args, 0)) {
            Some(mesh) if mesh.vertices.len() < MAX_VERTICES => {
                mesh.vertices.push(vertex);
                true
            }
            _ => false,
        }
    });
    Ok(Value::Bool(added))
}

pub(crate) fn native_ext_mesh_add_face(args: &[Value]) -> Result<Value, String> {
    let indices = [int_arg(args, 1), int_arg(args, 2), int_arg(args, 3)];
    let added = MESHES.with(|meshes| {
        let mut meshes = meshes.borrow_mut();
        match meshes.get_mut(&int_arg(args, 0)) {
            Some(mesh) if mesh.faces.len() < MAX_FACES => {
                let mut face = [0usize; 3];
                let mut valid = true;
                for (slot, index) in face.iter_mut().zip(indices.iter()) {
                    match usize::try_from(*index) {
                        Ok(value) if value < mesh.vertices.len() => *slot = value,
                        _ => valid = false,
                    }
                }
                if valid {
                    mesh.faces.push(face);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    });
    Ok(Value::Bool(added))
}

pub(crate) fn native_ext_mesh_vertex_count(args: &[Value]) -> Result<Value, String> {
    let count = MESHES.with(|meshes| {
        meshes
            .borrow()
            .get(&int_arg(args, 0))
            .map(|mesh| mesh.vertices.len() as i64)
            .unwrap_or(-1)
    });
    Ok(Value::Long(count))
}

pub(crate) fn native_ext_mesh_face_count(args: &[Value]) -> Result<Value, String> {
    let count = MESHES.with(|meshes| {
        meshes
            .borrow()
            .get(&int_arg(args, 0))
            .map(|mesh| mesh.faces.len() as i64)
            .unwrap_or(-1)
    });
    Ok(Value::Long(count))
}

pub(crate) fn native_ext_mesh_center(args: &[Value]) -> Result<Value, String> {
    let center = MESHES.with(|meshes| {
        meshes.borrow().get(&int_arg(args, 0)).map(|mesh| {
            if mesh.vertices.is_empty() {
                return [0.0, 0.0, 0.0];
            }
            let mut sum = [0.0; 3];
            for vertex in &mesh.vertices {
                sum[0] += vertex[0];
                sum[1] += vertex[1];
                sum[2] += vertex[2];
            }
            let count = mesh.vertices.len() as f64;
            [sum[0] / count, sum[1] / count, sum[2] / count]
        })
    });
    Ok(match center {
        Some(c) => Value::Array {
            elements: vec![Value::Double(c[0]), Value::Double(c[1]), Value::Double(c[2])],
        },
        None => Value::Null,
    })
}

pub(crate) fn native_ext_mesh_destroy(args: &[Value]) -> Result<Value, String> {
    let removed = MESHES.with(|meshes| meshes.borrow_mut().remove(&int_arg(args, 0)).is_some());
    Ok(Value::Bool(removed))
}

pub(crate) fn native_ext_lighting_diffuse(args: &[Value]) -> Result<Value, String> {
    let dot = f64_arg(args, 0) * f64_arg(args, 3)
        + f64_arg(args, 1) * f64_arg(args, 4)
        + f64_arg(args, 2) * f64_arg(args, 5);
    let ambient = f64_arg(args, 6);
    let intensity = f64_arg(args, 7);
    let mut value = ambient;
    if dot > 0.0 {
        value += intensity * dot;
    }
    Ok(Value::Double(value.clamp(0.0, 1.0)))
}

pub(crate) fn native_ext_midi_to_freq(args: &[Value]) -> Result<Value, String> {
    let note = int_arg(args, 0);
    let semitones = (note - 69) as f64 / 12.0;
    Ok(Value::Double(440.0 * 2f64.powf(semitones)))
}

pub(crate) fn native_ext_midi_note_name(args: &[Value]) -> Result<Value, String> {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let note = int_arg(args, 0);
    if !(0..=127).contains(&note) {
        return Ok(Value::String(Rc::new(String::new())));
    }
    let name = format!("{}{}", NAMES[note.rem_euclid(12) as usize], note / 12 - 1);
    Ok(Value::String(Rc::new(name)))
}

pub(crate) fn native_ext_filter_lowpass(args: &[Value]) -> Result<Value, String> {
    let previous = f64_arg(args, 0);
    let sample = f64_arg(args, 1);
    let alpha = f64_arg(args, 2).clamp(0.0, 1.0);
    Ok(Value::Double(previous + alpha * (sample - previous)))
}

pub(crate) fn native_ext_filter_highpass(args: &[Value]) -> Result<Value, String> {
    let previous = f64_arg(args, 0);
    let sample = f64_arg(args, 1);
    let alpha = f64_arg(args, 2).clamp(0.0, 1.0);
    Ok(Value::Double(sample - (previous + alpha * (sample - previous))))
}

/// Stable (name, function) table for the Phase 12 helper natives.
static EXT_NATIVES: &[(&str, NativeFn)] = &[
    ("ExtMesh_new", native_ext_mesh_new),
    ("ExtMesh_addVertex", native_ext_mesh_add_vertex),
    ("ExtMesh_addFace", native_ext_mesh_add_face),
    ("ExtMesh_vertexCount", native_ext_mesh_vertex_count),
    ("ExtMesh_faceCount", native_ext_mesh_face_count),
    ("ExtMesh_center", native_ext_mesh_center),
    ("ExtMesh_destroy", native_ext_mesh_destroy),
    ("ExtLighting_diffuse", native_ext_lighting_diffuse),
    ("ExtMidi_toFreq", native_ext_midi_to_freq),
    ("ExtMidi_noteName", native_ext_midi_note_name),
    ("ExtFilter_lowpass", native_ext_filter_lowpass),
    ("ExtFilter_highpass", native_ext_filter_highpass),
];

/// Direct lookup, usable as a fallback from a native dispatch function.
pub fn lookup_graphics_audio_ext(name: &str) -> Option<NativeFn> {
    EXT_NATIVES
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, function)| *function)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn expect_double(value: Result<Value, String>) -> f64 {
        match value {
            Ok(Value::Double(v)) => v,
            _ => panic!("expected Double"),
        }
    }

    fn expect_long(value: Result<Value, String>) -> i64 {
        match value {
            Ok(Value::Long(v)) => v,
            _ => panic!("expected Long"),
        }
    }

    fn expect_bool(value: Result<Value, String>) -> bool {
        match value {
            Ok(Value::Bool(v)) => v,
            _ => panic!("expected Bool"),
        }
    }

    fn expect_string(value: Result<Value, String>) -> String {
        match value {
            Ok(Value::String(v)) => v.as_str().to_string(),
            _ => panic!("expected String"),
        }
    }

    fn new_mesh() -> i64 {
        expect_long(native_ext_mesh_new(&[]))
    }

    #[test]
    fn ext_native_table_has_unique_names() {
        let mut seen = HashSet::new();
        for (name, _) in EXT_NATIVES {
            assert!(seen.insert(*name), "duplicate native name: {}", name);
            assert!(lookup_graphics_audio_ext(name).is_some());
        }
        assert!(lookup_graphics_audio_ext("ExtMesh_missing").is_none());
    }

    #[test]
    fn mesh_primitives_build_and_center_a_triangle() {
        let handle = new_mesh();
        let vertex = |x: f64, y: f64| -> [Value; 4] {
            [
                Value::Long(handle),
                Value::Double(x),
                Value::Double(y),
                Value::Double(0.0),
            ]
        };
        assert!(expect_bool(native_ext_mesh_add_vertex(&vertex(0.0, 0.0))));
        assert!(expect_bool(native_ext_mesh_add_vertex(&vertex(2.0, 0.0))));
        assert!(expect_bool(native_ext_mesh_add_vertex(&vertex(0.0, 2.0))));
        let face = [Value::Long(handle), Value::Int(0), Value::Int(1), Value::Int(2)];
        assert!(expect_bool(native_ext_mesh_add_face(&face)));
        let bad_face = [Value::Long(handle), Value::Int(0), Value::Int(1), Value::Int(9)];
        assert!(!expect_bool(native_ext_mesh_add_face(&bad_face)));
        assert_eq!(expect_long(native_ext_mesh_vertex_count(&[Value::Long(handle)])), 3);
        assert_eq!(expect_long(native_ext_mesh_face_count(&[Value::Long(handle)])), 1);
        match native_ext_mesh_center(&[Value::Long(handle)]) {
            Ok(Value::Array { elements }) => {
                assert_eq!(elements.len(), 3);
                assert_eq!(expect_double(Ok(elements[0].clone())), 2.0 / 3.0);
                assert_eq!(expect_double(Ok(elements[1].clone())), 2.0 / 3.0);
            }
            _ => panic!("expected center array"),
        }
        assert!(expect_bool(native_ext_mesh_destroy(&[Value::Long(handle)])));
        assert!(!expect_bool(native_ext_mesh_destroy(&[Value::Long(handle)])));
    }

    #[test]
    fn lighting_diffuse_is_clamped_and_ambient_is_kept() {
        let args = |nz: f64, ambient: f64, intensity: f64| -> [Value; 8] {
            [
                Value::Double(0.0),
                Value::Double(0.0),
                Value::Double(nz),
                Value::Double(0.0),
                Value::Double(0.0),
                Value::Double(-1.0),
                Value::Double(ambient),
                Value::Double(intensity),
            ]
        };
        assert_eq!(expect_double(native_ext_lighting_diffuse(&args(-1.0, 0.1, 0.5))), 0.6);
        assert_eq!(expect_double(native_ext_lighting_diffuse(&args(1.0, 0.1, 0.5))), 0.1);
        assert_eq!(expect_double(native_ext_lighting_diffuse(&args(-1.0, 0.9, 9.0))), 1.0);
    }

    #[test]
    fn midi_primitives_match_equal_temperament() {
        assert_eq!(expect_double(native_ext_midi_to_freq(&[Value::Int(69)])), 440.0);
        assert!((expect_double(native_ext_midi_to_freq(&[Value::Int(81)])) - 880.0).abs() < 1e-9);
        assert_eq!(expect_string(native_ext_midi_note_name(&[Value::Int(60)])), "C4");
        assert!(expect_string(native_ext_midi_note_name(&[Value::Int(200)])).is_empty());
    }

    #[test]
    fn filter_primitives_split_low_and_high_frequency_content() {
        let args = [Value::Double(0.0), Value::Double(1.0), Value::Double(0.5)];
        assert_eq!(expect_double(native_ext_filter_lowpass(&args)), 0.5);
        assert_eq!(expect_double(native_ext_filter_highpass(&args)), 0.5);
        let clamped = [
            Value::Double(0.0),
            Value::Double(1.0),
            Value::Double(4.0),
        ];
        assert_eq!(expect_double(native_ext_filter_lowpass(&clamped)), 1.0);
    }
}
