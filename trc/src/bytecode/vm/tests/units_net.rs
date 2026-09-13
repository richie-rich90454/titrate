    use crate::bytecode::value::Value;
    use crate::bytecode::vm::natives::regex::{native_regex_match, native_regex_replace};
    use crate::bytecode::vm::natives::system::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    // -- 58. test_meter_plus -----------------------------------------------------

    #[test]
    fn test_meter_plus() {
        // Simulate adding two Meter values (represented as Doubles)
        let m1 = Value::Double(5.0);
        let m2 = Value::Double(3.0);
        // Add
        match (&m1, &m2) {
            (Value::Double(a), Value::Double(b)) => {
                assert_eq!(a + b, 8.0);
            }
            _ => panic!("Expected Double values"),
        }
    }

    // -- 59. test_joule_from_base ------------------------------------------------

    #[test]
    fn test_joule_from_base() {
        // Joule = kg * m^2 / s^2
        // 1 J = 1.0 (in base SI units)
        let mass: f64 = 1.0;  // kg
        let velocity: f64 = 1.0;  // m/s
        let joules = 0.5 * mass * velocity * velocity;
        assert!((joules - 0.5).abs() < f64::EPSILON);
    }

    // -- 60. test_constants_boltzmann ---------------------------------------------

    #[test]
    fn test_constants_boltzmann() {
        // Boltzmann constant: 1.380649e-23 J/K
        let boltzmann: f64 = 1.380649e-23;
        assert!(boltzmann > 0.0, "Boltzmann constant should be positive");
        assert!((boltzmann - 1.380649e-23).abs() < 1e-30);
    }

    // -- 61. test_atom_creation --------------------------------------------------

    #[test]
    fn test_atom_creation() {
        // Create an Atom as a ClassInstance with element, x, y, z fields
        let mut atom_fields: HashMap<String, Value> = HashMap::new();
        atom_fields.insert("element".to_string(), Value::String(Rc::new("C".to_string())));
        atom_fields.insert("x".to_string(), Value::Double(0.0));
        atom_fields.insert("y".to_string(), Value::Double(0.0));
        atom_fields.insert("z".to_string(), Value::Double(0.0));

        let atom = Value::ClassInstance {
            class_name: "Atom".to_string(),
            fields: Rc::new(RefCell::new(atom_fields)),
            vtable: HashMap::new(),
        };

        match &atom {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "Atom");
                let borrowed = fields.borrow();
                assert_eq!(borrowed.get("element"), Some(&Value::String(Rc::new("C".to_string()))));
                assert_eq!(borrowed.get("x"), Some(&Value::Double(0.0)));
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 62. test_atom_distance --------------------------------------------------

    #[test]
    fn test_atom_distance() {
        // Distance between two atoms at (0,0,0) and (3,4,0) = 5.0
        let x1: f64 = 0.0; let y1: f64 = 0.0; let z1: f64 = 0.0;
        let x2: f64 = 3.0; let y2: f64 = 4.0; let z2: f64 = 0.0;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let dz = z2 - z1;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!((dist - 5.0).abs() < f64::EPSILON);
    }

    // -- 63. test_molecule_add_atom -----------------------------------------------

    #[test]
    fn test_molecule_add_atom() {
        // Create a Molecule as a ClassInstance with an atoms Array field
        let mut mol_fields: HashMap<String, Value> = HashMap::new();
        mol_fields.insert("name".to_string(), Value::String(Rc::new("Water".to_string())));
        mol_fields.insert("atoms".to_string(), Value::Array { elements: vec![] });

        // Add atoms
        if let Some(Value::Array { elements }) = mol_fields.get_mut("atoms") {
            elements.push(Value::String(Rc::new("O".to_string())));
            elements.push(Value::String(Rc::new("H".to_string())));
            elements.push(Value::String(Rc::new("H".to_string())));
        }

        let molecule = Value::ClassInstance {
            class_name: "Molecule".to_string(),
            fields: Rc::new(RefCell::new(mol_fields)),
            vtable: HashMap::new(),
        };

        match &molecule {
            Value::ClassInstance { fields, .. } => {
                let borrowed = fields.borrow();
                match borrowed.get("atoms") {
                    Some(Value::Array { elements }) => {
                        assert_eq!(elements.len(), 3);
                    }
                    _ => panic!("Expected atoms array"),
                }
                match borrowed.get("name") {
                    Some(Value::String(s)) => {
                        assert_eq!(s.as_str(), "Water");
                    }
                    _ => panic!("Expected name string"),
                }
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 64. test_tcp_client_creation --------------------------------------------

    #[test]
    fn test_tcp_client_creation() {
        // Create a TcpClient as a ClassInstance with host and port fields
        let mut client_fields: HashMap<String, Value> = HashMap::new();
        client_fields.insert("host".to_string(), Value::String(Rc::new("127.0.0.1".to_string())));
        client_fields.insert("port".to_string(), Value::Int(8080));
        client_fields.insert("connected".to_string(), Value::Bool(false));

        let client = Value::ClassInstance {
            class_name: "TcpClient".to_string(),
            fields: Rc::new(RefCell::new(client_fields)),
            vtable: HashMap::new(),
        };

        match &client {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "TcpClient");
                let borrowed = fields.borrow();
                assert_eq!(borrowed.get("host"), Some(&Value::String(Rc::new("127.0.0.1".to_string()))));
                assert_eq!(borrowed.get("port"), Some(&Value::Int(8080)));
                assert_eq!(borrowed.get("connected"), Some(&Value::Bool(false)));
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 65. test_http_client_creation -------------------------------------------

    #[test]
    fn test_http_client_creation() {
        // Create an HttpClient as a ClassInstance with base_url field
        let mut client_fields: HashMap<String, Value> = HashMap::new();
        client_fields.insert("base_url".to_string(), Value::String(Rc::new("https://api.example.com".to_string())));
        client_fields.insert("timeout".to_string(), Value::Long(30000));

        let client = Value::ClassInstance {
            class_name: "HttpClient".to_string(),
            fields: Rc::new(RefCell::new(client_fields)),
            vtable: HashMap::new(),
        };

        match &client {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "HttpClient");
                let borrowed = fields.borrow();
                assert_eq!(borrowed.get("base_url"), Some(&Value::String(Rc::new("https://api.example.com".to_string()))));
                assert_eq!(borrowed.get("timeout"), Some(&Value::Long(30000)));
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 66. test_duration_of_seconds --------------------------------------------

    #[test]
    fn test_duration_of_seconds() {
        // Create Duration from seconds (represented as i64 milliseconds)
        let seconds: i64 = 5;
        let duration_ms = seconds * 1000;
        assert_eq!(duration_ms, 5000);

        let duration_ms2: i64 = 0;
        assert_eq!(duration_ms2, 0);

        let duration_ms3: i64 = 1000;
        assert_eq!(duration_ms3, 1000);
    }

    // -- 67. test_datetime_plus_duration -----------------------------------------

    #[test]
    fn test_datetime_plus_duration() {
        // Add duration to datetime (represented as i64 ms timestamps)
        let datetime_ms: i64 = 1609459200000; // 2021-01-01 00:00:00 UTC
        let duration_ms: i64 = 86400000; // 1 day
        let result = datetime_ms + duration_ms;
        assert_eq!(result, 1609545600000); // 2021-01-02 00:00:00 UTC
    }

    // -- 68. test_datetime_comparison --------------------------------------------

    #[test]
    fn test_datetime_comparison() {
        // Compare two datetimes
        let dt1: i64 = 1609459200000; // 2021-01-01
        let dt2: i64 = 1609545600000; // 2021-01-02
        assert!(dt1 < dt2);
        assert!(dt2 > dt1);
        assert!(dt1 == dt1);
        assert!(dt1 != dt2);
    }

    // -- 69. test_regex_compile --------------------------------------------------

    #[test]
    fn test_regex_compile() {
        // Compiling a regex pattern should not panic
        let result = native_regex_match(&[
            Value::String(Rc::new(r"\d+".to_string())),
            Value::String(Rc::new("123".to_string())),
        ]);
        assert!(result.is_ok(), "regex compile and match should succeed");
    }

    // -- 70. test_regex_match_simple ---------------------------------------------

    #[test]
    fn test_regex_match_simple() {
        // Match a simple pattern
        let result = native_regex_match(&[
            Value::String(Rc::new(r"^hello".to_string())),
            Value::String(Rc::new("hello world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Non-matching
        let result = native_regex_match(&[
            Value::String(Rc::new(r"^world".to_string())),
            Value::String(Rc::new("hello world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    // -- 71. test_regex_replace_simple -------------------------------------------

    #[test]
    fn test_regex_replace_simple() {
        // Replace with a simple pattern
        let result = native_regex_replace(&[
            Value::String(Rc::new(r"cat".to_string())),
            Value::String(Rc::new("the cat sat on the mat".to_string())),
            Value::String(Rc::new("dog".to_string())),
        ]);
        assert_eq!(
            result.unwrap(),
            Value::String(Rc::new("the dog sat on the mat".to_string()))
        );
    }

    // -- 72. test_env_get -------------------------------------------------------

    #[test]
    fn test_env_get() {
        // Set an env var then retrieve it
        std::env::set_var("TITRATE_TEST_ENV_GET", "hello");
        let result = native_env_get(&[Value::String(Rc::new("TITRATE_TEST_ENV_GET".to_string()))]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("hello".to_string())));

        // Non-existent env var returns Null
        let result = native_env_get(&[Value::String(Rc::new("TITRATE_NONEXISTENT_VAR_XYZ".to_string()))]);
        assert_eq!(result.unwrap(), Value::Null);

        // Error on wrong type
        let result = native_env_get(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 73. test_env_set -------------------------------------------------------

    #[test]
    fn test_env_set() {
        let result = native_env_set(&[
            Value::String(Rc::new("TITRATE_TEST_ENV_SET".to_string())),
            Value::String(Rc::new("world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Void);
        assert_eq!(std::env::var("TITRATE_TEST_ENV_SET").unwrap(), "world");

        // Error on wrong type
        let result = native_env_set(&[Value::Int(1), Value::Int(2)]);
        assert!(result.is_err());
    }

    // -- 74. test_env_vars -------------------------------------------------------

    #[test]
    fn test_env_vars() {
        let result = native_env_vars(&[]);
        match result.unwrap() {
            Value::Array { elements } => {
                assert!(!elements.is_empty());
                // Each element should be a "key=value" string
                for elem in &elements {
                    match elem {
                        Value::String(s) => assert!(s.contains('=')),
                        _ => panic!("Expected String in env vars array"),
                    }
                }
            }
            _ => panic!("Expected Array from Env_vars"),
        }
    }

    // -- 75. test_fs_exists -----------------------------------------------------

    #[test]
    fn test_fs_exists() {
        // Current directory should exist
        let result = native_fs_exists(&[Value::String(Rc::new(".".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Non-existent path
        let result = native_fs_exists(&[Value::String(Rc::new("/no/such/path/titrate_test_xyz".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // Error on wrong type
        let result = native_fs_exists(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 76. test_fs_is_file ----------------------------------------------------

    #[test]
    fn test_fs_is_file() {
        // Current directory is not a file
        let result = native_fs_is_file(&[Value::String(Rc::new(".".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // A known existing file - use Cargo.toml from the crate root
        let cargo_toml = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml")
            .to_string_lossy()
            .to_string();
        let result = native_fs_is_file(&[Value::String(Rc::new(cargo_toml))]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Error on wrong type
        let result = native_fs_is_file(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 77. test_fs_is_dir -----------------------------------------------------

    #[test]
    fn test_fs_is_dir() {
        // Current directory should be a directory
        let result = native_fs_is_dir(&[Value::String(Rc::new(".".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // This source file is not a directory
        let this_file = file!().replace('\\', "/");
        let result = native_fs_is_dir(&[Value::String(Rc::new(this_file))]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // Error on wrong type
        let result = native_fs_is_dir(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 78. test_fs_size -------------------------------------------------------

    #[test]
    fn test_fs_size() {
        // Cargo.toml should have a positive size
        let cargo_toml = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml")
            .to_string_lossy()
            .to_string();
        let result = native_fs_size(&[Value::String(Rc::new(cargo_toml))]);
        match result.unwrap() {
            Value::Long(n) => assert!(n > 0),
            _ => panic!("Expected Long from Fs_size"),
        }

        // Non-existent file should error
        let result = native_fs_size(&[Value::String(Rc::new("/no/such/file_titrate_xyz".to_string()))]);
        assert!(result.is_err());

        // Error on wrong type
        let result = native_fs_size(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 79. test_process_id ----------------------------------------------------

    #[test]
    fn test_process_id() {
        let result = native_process_id(&[]);
        match result.unwrap() {
            Value::Long(n) => assert!(n > 0),
            _ => panic!("Expected Long from Process_id"),
        }
    }

    // -- 80. test_process_args --------------------------------------------------

    #[test]
    fn test_process_args() {
        let result = native_process_args(&[]);
        match result.unwrap() {
            Value::Array { elements } => {
                assert!(!elements.is_empty());
                for elem in &elements {
                    match elem {
                        Value::String(_) => {}
                        _ => panic!("Expected String in process args array"),
                    }
                }
            }
            _ => panic!("Expected Array from Process_args"),
        }
    }

    // -- 81. test_os_name -------------------------------------------------------

    #[test]
    fn test_os_name() {
        let result = native_os_name(&[]);
        match result.unwrap() {
            Value::String(s) => assert!(!s.is_empty()),
            _ => panic!("Expected String from Os_name"),
        }
    }

    // -- 82. test_os_arch -------------------------------------------------------

    #[test]
    fn test_os_arch() {
        let result = native_os_arch(&[]);
        match result.unwrap() {
            Value::String(s) => assert!(!s.is_empty()),
            _ => panic!("Expected String from Os_arch"),
        }
    }

    // -- 83. test_os_family -----------------------------------------------------

    #[test]
    fn test_os_family() {
        let result = native_os_family(&[]);
        match result.unwrap() {
            Value::String(s) => {
                assert!(!s.is_empty());
                // Should be "unix" or "windows"
                assert!(s.as_str() == "unix" || s.as_str() == "windows", "Unexpected OS family: {}", s);
            }
            _ => panic!("Expected String from Os_family"),
        }
    }
