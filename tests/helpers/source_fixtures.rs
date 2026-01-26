//! Common source code fixtures for tests.

// Simple definitions
pub const SIMPLE_PART_DEF: &str = "part def Vehicle;";
pub const SIMPLE_PORT_DEF: &str = "port def DataPort;";
pub const SIMPLE_ACTION_DEF: &str = "action def Move;";
pub const SIMPLE_ITEM_DEF: &str = "item def Payload;";
pub const SIMPLE_ATTRIBUTE_DEF: &str = "attribute def Mass;";

pub const MULTIPLE_DEFINITIONS: &str = r#"
part def Vehicle;
part def Car;
part def Truck;
"#;

// Nested structures
pub const NESTED_PACKAGE: &str = r#"
package Vehicles {
    part def Vehicle;
    part def Car;
}
"#;

pub const DEEPLY_NESTED_PACKAGES: &str = r#"
package Level1 {
    package Level2 {
        package Level3 {
            part def DeepPart;
        }
    }
}
"#;

pub const PART_WITH_USAGES: &str = r#"
part def Vehicle {
    part engine : Engine;
    part wheels : Wheel[4];
    attribute mass : Real;
}
part def Engine;
part def Wheel;
"#;

// Specialization
pub const SIMPLE_SPECIALIZATION: &str = r#"
part def Vehicle;
part def Car :> Vehicle;
"#;

pub const SPECIALIZATION_CHAIN: &str = r#"
part def Thing;
part def Vehicle :> Thing;
part def Car :> Vehicle;
part def SportsCar :> Car;
"#;

pub const MULTIPLE_SPECIALIZATION: &str = r#"
part def Driveable;
part def Flyable;
part def FlyingCar :> Driveable, Flyable;
"#;

// Imports
pub const WILDCARD_IMPORT: &str = r#"
package Base {
    part def Vehicle;
    part def Engine;
}
package Derived {
    public import Base::*;
    part myCar : Vehicle;
}
"#;

pub const MEMBER_IMPORT: &str = r#"
package Base {
    part def Vehicle;
    part def Engine;
}
package Derived {
    import Base::Vehicle;
    part myCar : Vehicle;
}
"#;

pub const NESTED_IMPORTS: &str = r#"
package Definitions {
    public import PartDefinitions::*;
    package PartDefinitions {
        part def Vehicle;
    }
}
package Usage {
    import Definitions::*;
    part car : Vehicle;
}
"#;

// Type references
pub const TYPED_USAGE: &str = r#"
part def Vehicle;
part myCar : Vehicle;
"#;

pub const REDEFINITION: &str = r#"
part def Base {
    part inner;
}
part def Derived :> Base {
    part :>> inner;
}
"#;

// Helper functions
pub fn package_with_n_parts(n: usize) -> String {
    let mut parts = String::new();
    for i in 0..n {
        parts.push_str(&format!("    part def Part{};\n", i));
    }
    format!("package Generated {{\n{}}}", parts)
}

pub fn nested_packages(depth: usize) -> String {
    let mut result = String::new();
    let mut indent = String::new();

    for i in 0..depth {
        result.push_str(&format!("{}package Level{} {{\n", indent, i + 1));
        indent.push_str("    ");
    }

    result.push_str(&format!("{}part def DeepPart;\n", indent));

    for _ in 0..depth {
        indent.pop();
        indent.pop();
        indent.pop();
        indent.pop();
        result.push_str(&format!("{}}}\n", indent));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_with_n_parts() {
        let source = package_with_n_parts(3);
        assert!(source.contains("Part0"));
        assert!(source.contains("Part1"));
        assert!(source.contains("Part2"));
        assert!(!source.contains("Part3"));
    }

    #[test]
    fn test_nested_packages() {
        let source = nested_packages(3);
        assert!(source.contains("Level1"));
        assert!(source.contains("Level2"));
        assert!(source.contains("Level3"));
        assert!(source.contains("DeepPart"));
    }
}
