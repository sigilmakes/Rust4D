//! Math bindings for Lua
//!
//! Provides Lua access to 4D math types:
//! - `Vec4` - 4D vector with operator overloading
//! - `Rotor4` - 4D rotation (geometric algebra)
//! - `Transform4D` - Position, rotation, scale
//!
//! ## Usage (Lua)
//!
//! ```lua
//! -- Vec4 creation and operations
//! local v = Vec4.new(1, 2, 3, 4)
//! local u = Vec4.new(4, 3, 2, 1)
//! local sum = v + u
//! local scaled = v * 2.5
//! print(v:length(), v:dot(u))
//! print(v.x, v.y, v.z, v.w)
//!
//! -- Rotor4 for rotations
//! local r = Rotor4.from_plane("XY", math.pi / 2)
//! local rotated = r:rotate(v)
//!
//! -- Transform4D for full transforms
//! local t = Transform4D.from_position(Vec4.new(10, 0, 0, 0))
//! local world_pos = t:transform_point(v)
//! ```
//!
//! This module is owned by Agent D2 (Math/Physics Bindings).

use mlua::prelude::*;
use rust4d_math::{Rotor4, RotationPlane, Vec4};

// Re-export Transform4D binding if rust4d_core is available
// For now, we'll implement Transform4D bindings directly using the math crate

/// Lua wrapper for Vec4
///
/// Provides full access to 4D vector operations with operator overloading.
#[derive(Clone, Copy, Debug)]
pub struct LuaVec4(pub Vec4);

impl LuaUserData for LuaVec4 {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Component access: v.x, v.y, v.z, v.w
        fields.add_field_method_get("x", |_, this| Ok(this.0.x));
        fields.add_field_method_set("x", |_, this, val: f32| {
            this.0.x = val;
            Ok(())
        });

        fields.add_field_method_get("y", |_, this| Ok(this.0.y));
        fields.add_field_method_set("y", |_, this, val: f32| {
            this.0.y = val;
            Ok(())
        });

        fields.add_field_method_get("z", |_, this| Ok(this.0.z));
        fields.add_field_method_set("z", |_, this, val: f32| {
            this.0.z = val;
            Ok(())
        });

        fields.add_field_method_get("w", |_, this| Ok(this.0.w));
        fields.add_field_method_set("w", |_, this, val: f32| {
            this.0.w = val;
            Ok(())
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Dot product
        methods.add_method("dot", |_, this, other: LuaVec4| Ok(this.0.dot(other.0)));

        // Length (magnitude)
        methods.add_method("length", |_, this, ()| Ok(this.0.length()));

        // Length squared (faster, avoids sqrt)
        methods.add_method("length_squared", |_, this, ()| Ok(this.0.length_squared()));

        // Normalize to unit length
        methods.add_method("normalized", |_, this, ()| {
            Ok(LuaVec4(this.0.normalized()))
        });

        // Distance between two vectors
        methods.add_method("distance", |_, this, other: LuaVec4| {
            Ok(this.0.distance(other.0))
        });

        // Distance squared (faster, avoids sqrt)
        methods.add_method("distance_squared", |_, this, other: LuaVec4| {
            Ok(this.0.distance_squared(other.0))
        });

        // Linear interpolation
        methods.add_method("lerp", |_, this, (other, t): (LuaVec4, f32)| {
            Ok(LuaVec4(this.0.lerp(other.0, t)))
        });

        // Component-wise absolute value
        methods.add_method("abs", |_, this, ()| Ok(LuaVec4(this.0.abs())));

        // Component-wise sign
        methods.add_method("sign", |_, this, ()| Ok(LuaVec4(this.0.sign())));

        // Component-wise multiplication (Hadamard product)
        methods.add_method("component_mul", |_, this, other: LuaVec4| {
            Ok(LuaVec4(this.0.component_mul(other.0)))
        });

        // Component-wise min
        methods.add_method("min_components", |_, this, other: LuaVec4| {
            Ok(LuaVec4(this.0.min_components(other.0)))
        });

        // Component-wise max
        methods.add_method("max_components", |_, this, other: LuaVec4| {
            Ok(LuaVec4(this.0.max_components(other.0)))
        });

        // Clamp components
        methods.add_method(
            "clamp_components",
            |_, this, (min, max): (LuaVec4, LuaVec4)| {
                Ok(LuaVec4(this.0.clamp_components(min.0, max.0)))
            },
        );

        // Extract xyz as array (for 3D interop)
        methods.add_method("xyz", |lua, this, ()| {
            let arr = this.0.xyz();
            let table = lua.create_table()?;
            table.set(1, arr[0])?;
            table.set(2, arr[1])?;
            table.set(3, arr[2])?;
            Ok(table)
        });

        // Operator overloading via metamethods

        // Addition: v1 + v2
        methods.add_meta_method(LuaMetaMethod::Add, |_, this, other: LuaVec4| {
            Ok(LuaVec4(this.0 + other.0))
        });

        // Subtraction: v1 - v2
        methods.add_meta_method(LuaMetaMethod::Sub, |_, this, other: LuaVec4| {
            Ok(LuaVec4(this.0 - other.0))
        });

        // Scalar multiplication: v * scalar
        methods.add_meta_method(LuaMetaMethod::Mul, |_, this, scalar: f32| {
            Ok(LuaVec4(this.0 * scalar))
        });

        // Scalar division: v / scalar
        methods.add_meta_method(LuaMetaMethod::Div, |_, this, scalar: f32| {
            Ok(LuaVec4(this.0 / scalar))
        });

        // Unary minus: -v
        methods.add_meta_method(LuaMetaMethod::Unm, |_, this, ()| Ok(LuaVec4(-this.0)));

        // Equality: v1 == v2
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaVec4| {
            Ok(this.0 == other.0)
        });

        // String representation: tostring(v)
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "Vec4({}, {}, {}, {})",
                this.0.x, this.0.y, this.0.z, this.0.w
            ))
        });
    }
}

impl FromLua for LuaVec4 {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        match value {
            LuaValue::UserData(ud) => ud.borrow::<LuaVec4>().map(|v| *v),
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "Vec4".to_string(),
                message: Some("expected Vec4 userdata".to_string()),
            }),
        }
    }
}

/// Lua wrapper for Rotor4
///
/// Provides 4D rotation operations using geometric algebra.
#[derive(Clone, Copy, Debug)]
pub struct LuaRotor4(pub Rotor4);

impl LuaUserData for LuaRotor4 {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Expose rotor components (mostly for debugging)
        fields.add_field_method_get("s", |_, this| Ok(this.0.s));
        fields.add_field_method_get("b_xy", |_, this| Ok(this.0.b_xy));
        fields.add_field_method_get("b_xz", |_, this| Ok(this.0.b_xz));
        fields.add_field_method_get("b_xw", |_, this| Ok(this.0.b_xw));
        fields.add_field_method_get("b_yz", |_, this| Ok(this.0.b_yz));
        fields.add_field_method_get("b_yw", |_, this| Ok(this.0.b_yw));
        fields.add_field_method_get("b_zw", |_, this| Ok(this.0.b_zw));
        fields.add_field_method_get("p", |_, this| Ok(this.0.p));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Rotate a vector: r:rotate(v) -> Vec4
        methods.add_method("rotate", |_, this, v: LuaVec4| {
            Ok(LuaVec4(this.0.rotate(v.0)))
        });

        // Get the reverse (conjugate) of the rotor
        methods.add_method("reverse", |_, this, ()| Ok(LuaRotor4(this.0.reverse())));

        // Normalize the rotor
        methods.add_method("normalize", |_, this, ()| {
            Ok(LuaRotor4(this.0.normalize()))
        });

        // Get magnitude
        methods.add_method("magnitude", |_, this, ()| Ok(this.0.magnitude()));

        // Convert to rotation matrix (4x4)
        methods.add_method("to_matrix", |lua, this, ()| {
            let m = this.0.to_matrix();
            let table = lua.create_table()?;
            for (i, row) in m.iter().enumerate() {
                let row_table = lua.create_table()?;
                for (j, val) in row.iter().enumerate() {
                    row_table.set(j + 1, *val)?;
                }
                table.set(i + 1, row_table)?;
            }
            Ok(table)
        });

        // Rotor composition: r1 * r2
        methods.add_meta_method(LuaMetaMethod::Mul, |_, this, other: LuaRotor4| {
            Ok(LuaRotor4(this.0.compose(&other.0)))
        });

        // String representation
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "Rotor4(s={:.3}, xy={:.3}, xz={:.3}, xw={:.3}, yz={:.3}, yw={:.3}, zw={:.3}, p={:.3})",
                this.0.s, this.0.b_xy, this.0.b_xz, this.0.b_xw,
                this.0.b_yz, this.0.b_yw, this.0.b_zw, this.0.p
            ))
        });
    }
}

impl FromLua for LuaRotor4 {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        match value {
            LuaValue::UserData(ud) => ud.borrow::<LuaRotor4>().map(|r| *r),
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "Rotor4".to_string(),
                message: Some("expected Rotor4 userdata".to_string()),
            }),
        }
    }
}

/// Lua wrapper for Transform4D
///
/// Provides full 4D transformation with position, rotation, and scale.
#[derive(Clone, Copy, Debug)]
pub struct LuaTransform4D {
    pub position: Vec4,
    pub rotation: Rotor4,
    pub scale: f32,
}

impl LuaTransform4D {
    /// Create an identity transform
    pub fn identity() -> Self {
        Self {
            position: Vec4::ZERO,
            rotation: Rotor4::IDENTITY,
            scale: 1.0,
        }
    }

    /// Create from position only
    pub fn from_position(position: Vec4) -> Self {
        Self {
            position,
            rotation: Rotor4::IDENTITY,
            scale: 1.0,
        }
    }

    /// Transform a point from local to world space
    pub fn transform_point(&self, p: Vec4) -> Vec4 {
        let scaled = p * self.scale;
        let rotated = self.rotation.rotate(scaled);
        rotated + self.position
    }

    /// Transform a direction (no translation)
    pub fn transform_direction(&self, d: Vec4) -> Vec4 {
        let scaled = d * self.scale;
        self.rotation.rotate(scaled)
    }
}

impl LuaUserData for LuaTransform4D {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Position as Vec4
        fields.add_field_method_get("position", |_, this| Ok(LuaVec4(this.position)));
        fields.add_field_method_set("position", |_, this, pos: LuaVec4| {
            this.position = pos.0;
            Ok(())
        });

        // Rotation as Rotor4
        fields.add_field_method_get("rotation", |_, this| Ok(LuaRotor4(this.rotation)));
        fields.add_field_method_set("rotation", |_, this, rot: LuaRotor4| {
            this.rotation = rot.0;
            Ok(())
        });

        // Scale
        fields.add_field_method_get("scale", |_, this| Ok(this.scale));
        fields.add_field_method_set("scale", |_, this, scale: f32| {
            this.scale = scale;
            Ok(())
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Transform a point from local to world space
        methods.add_method("transform_point", |_, this, p: LuaVec4| {
            Ok(LuaVec4(this.transform_point(p.0)))
        });

        // Transform a direction (no translation)
        methods.add_method("transform_direction", |_, this, d: LuaVec4| {
            Ok(LuaVec4(this.transform_direction(d.0)))
        });

        // Translate by offset
        methods.add_method_mut("translate", |_, this, offset: LuaVec4| {
            this.position = this.position + offset.0;
            Ok(())
        });

        // Rotate by rotor
        methods.add_method_mut("rotate", |_, this, rotor: LuaRotor4| {
            this.rotation = rotor.0.compose(&this.rotation).normalize();
            Ok(())
        });

        // String representation
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "Transform4D(pos=({:.2}, {:.2}, {:.2}, {:.2}), scale={:.2})",
                this.position.x, this.position.y, this.position.z, this.position.w, this.scale
            ))
        });
    }
}

impl FromLua for LuaTransform4D {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        match value {
            LuaValue::UserData(ud) => ud.borrow::<LuaTransform4D>().map(|t| *t),
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "Transform4D".to_string(),
                message: Some("expected Transform4D userdata".to_string()),
            }),
        }
    }
}

/// Parse a rotation plane name string to RotationPlane enum
fn parse_rotation_plane(name: &str) -> LuaResult<RotationPlane> {
    match name.to_uppercase().as_str() {
        "XY" => Ok(RotationPlane::XY),
        "XZ" => Ok(RotationPlane::XZ),
        "XW" => Ok(RotationPlane::XW),
        "YZ" => Ok(RotationPlane::YZ),
        "YW" => Ok(RotationPlane::YW),
        "ZW" => Ok(RotationPlane::ZW),
        _ => Err(LuaError::RuntimeError(format!(
            "Invalid rotation plane '{}'. Valid planes: XY, XZ, XW, YZ, YW, ZW",
            name
        ))),
    }
}

/// Register all math bindings with the Lua VM
///
/// Creates global tables for:
/// - `Vec4` with constructor `Vec4.new(x, y, z, w)` and constants
/// - `Rotor4` with constructors `Rotor4.identity()`, `Rotor4.from_plane(plane, angle)`
/// - `Transform4D` with constructors `Transform4D.identity()`, `Transform4D.from_position(vec4)`
pub fn register(lua: &Lua) -> LuaResult<()> {
    // === Vec4 ===
    let vec4_table = lua.create_table()?;

    // Vec4.new(x, y, z, w)
    vec4_table.set(
        "new",
        lua.create_function(|_, (x, y, z, w): (f32, f32, f32, f32)| {
            Ok(LuaVec4(Vec4::new(x, y, z, w)))
        })?,
    )?;

    // Vec4.zero()
    vec4_table.set(
        "zero",
        lua.create_function(|_, ()| Ok(LuaVec4(Vec4::ZERO)))?,
    )?;

    // Vec4 constants
    vec4_table.set("ZERO", LuaVec4(Vec4::ZERO))?;
    vec4_table.set("X", LuaVec4(Vec4::X))?;
    vec4_table.set("Y", LuaVec4(Vec4::Y))?;
    vec4_table.set("Z", LuaVec4(Vec4::Z))?;
    vec4_table.set("W", LuaVec4(Vec4::W))?;

    lua.globals().set("Vec4", vec4_table)?;

    // === Rotor4 ===
    let rotor4_table = lua.create_table()?;

    // Rotor4.identity()
    rotor4_table.set(
        "identity",
        lua.create_function(|_, ()| Ok(LuaRotor4(Rotor4::IDENTITY)))?,
    )?;

    // Rotor4.from_plane(plane_name, angle_radians)
    // plane_name: "XY", "XZ", "XW", "YZ", "YW", "ZW"
    rotor4_table.set(
        "from_plane",
        lua.create_function(|_, (plane_name, angle): (String, f32)| {
            let plane = parse_rotation_plane(&plane_name)?;
            Ok(LuaRotor4(Rotor4::from_plane_angle(plane, angle)))
        })?,
    )?;

    // Rotor4.from_euler_xyz(x, y, z) - 3D Euler angles (radians)
    rotor4_table.set(
        "from_euler_xyz",
        lua.create_function(|_, (x, y, z): (f32, f32, f32)| {
            Ok(LuaRotor4(Rotor4::from_euler_xyz(x, y, z)))
        })?,
    )?;

    // Rotor4.from_plane_vectors(a, b, angle) - rotation in plane spanned by two vectors
    rotor4_table.set(
        "from_plane_vectors",
        lua.create_function(|_, (a, b, angle): (LuaVec4, LuaVec4, f32)| {
            Ok(LuaRotor4(Rotor4::from_plane_vectors(a.0, b.0, angle)))
        })?,
    )?;

    // Rotor4.IDENTITY constant
    rotor4_table.set("IDENTITY", LuaRotor4(Rotor4::IDENTITY))?;

    lua.globals().set("Rotor4", rotor4_table)?;

    // === Transform4D ===
    let transform_table = lua.create_table()?;

    // Transform4D.identity()
    transform_table.set(
        "identity",
        lua.create_function(|_, ()| Ok(LuaTransform4D::identity()))?,
    )?;

    // Transform4D.from_position(vec4)
    transform_table.set(
        "from_position",
        lua.create_function(|_, pos: LuaVec4| Ok(LuaTransform4D::from_position(pos.0)))?,
    )?;

    // Transform4D.from_position_rotation(vec4, rotor4)
    transform_table.set(
        "from_position_rotation",
        lua.create_function(|_, (pos, rot): (LuaVec4, LuaRotor4)| {
            Ok(LuaTransform4D {
                position: pos.0,
                rotation: rot.0,
                scale: 1.0,
            })
        })?,
    )?;

    lua.globals().set("Transform4D", transform_table)?;

    log::debug!("[math] Math bindings registered (Vec4, Rotor4, Transform4D)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_lua_with_math() -> Lua {
        let lua = Lua::new();
        register(&lua).expect("Failed to register math bindings");
        lua
    }

    // === Vec4 tests ===

    #[test]
    fn test_vec4_constructor() {
        let lua = create_lua_with_math();
        let result: (f32, f32, f32, f32) = lua
            .load(
                r#"
            local v = Vec4.new(1, 2, 3, 4)
            return v.x, v.y, v.z, v.w
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(result, (1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_vec4_constants() {
        let lua = create_lua_with_math();
        let result: (f32, f32, f32, f32) = lua
            .load(
                r#"
            return Vec4.X.x, Vec4.Y.y, Vec4.Z.z, Vec4.W.w
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(result, (1.0, 1.0, 1.0, 1.0));
    }

    #[test]
    fn test_vec4_addition() {
        let lua = create_lua_with_math();
        let x: f32 = lua
            .load(
                r#"
            local a = Vec4.new(1, 2, 3, 0)
            local b = Vec4.new(4, 5, 6, 0)
            local sum = a + b
            return sum.x
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, 5.0);
    }

    #[test]
    fn test_vec4_subtraction() {
        let lua = create_lua_with_math();
        let x: f32 = lua
            .load(
                r#"
            local a = Vec4.new(5, 5, 5, 5)
            local b = Vec4.new(1, 2, 3, 4)
            local diff = a - b
            return diff.x
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, 4.0);
    }

    #[test]
    fn test_vec4_scalar_mul() {
        let lua = create_lua_with_math();
        let x: f32 = lua
            .load(
                r#"
            local v = Vec4.new(2, 3, 4, 5)
            local scaled = v * 2
            return scaled.x
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, 4.0);
    }

    #[test]
    fn test_vec4_negation() {
        let lua = create_lua_with_math();
        let x: f32 = lua
            .load(
                r#"
            local v = Vec4.new(1, 2, 3, 4)
            local neg = -v
            return neg.x
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, -1.0);
    }

    #[test]
    fn test_vec4_length() {
        let lua = create_lua_with_math();
        let len: f32 = lua
            .load(
                r#"
            local v = Vec4.new(3, 4, 0, 0)
            return v:length()
        "#,
            )
            .eval()
            .unwrap();
        assert!((len - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_vec4_dot() {
        let lua = create_lua_with_math();
        let dot: f32 = lua
            .load(
                r#"
            local a = Vec4.new(1, 0, 0, 0)
            local b = Vec4.new(1, 0, 0, 0)
            return a:dot(b)
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(dot, 1.0);
    }

    #[test]
    fn test_vec4_normalized() {
        let lua = create_lua_with_math();
        let len: f32 = lua
            .load(
                r#"
            local v = Vec4.new(10, 0, 0, 0)
            local n = v:normalized()
            return n:length()
        "#,
            )
            .eval()
            .unwrap();
        assert!((len - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_vec4_distance() {
        let lua = create_lua_with_math();
        let dist: f32 = lua
            .load(
                r#"
            local a = Vec4.new(0, 0, 0, 0)
            local b = Vec4.new(3, 4, 0, 0)
            return a:distance(b)
        "#,
            )
            .eval()
            .unwrap();
        assert!((dist - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_vec4_lerp() {
        let lua = create_lua_with_math();
        let x: f32 = lua
            .load(
                r#"
            local a = Vec4.new(0, 0, 0, 0)
            local b = Vec4.new(10, 10, 10, 10)
            local mid = a:lerp(b, 0.5)
            return mid.x
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, 5.0);
    }

    #[test]
    fn test_vec4_field_set() {
        let lua = create_lua_with_math();
        let x: f32 = lua
            .load(
                r#"
            local v = Vec4.new(1, 2, 3, 4)
            v.x = 100
            return v.x
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, 100.0);
    }

    #[test]
    fn test_vec4_tostring() {
        let lua = create_lua_with_math();
        let s: String = lua
            .load(
                r#"
            local v = Vec4.new(1, 2, 3, 4)
            return tostring(v)
        "#,
            )
            .eval()
            .unwrap();
        assert!(s.contains("Vec4"));
        assert!(s.contains("1"));
    }

    #[test]
    fn test_vec4_equality() {
        let lua = create_lua_with_math();
        let eq: bool = lua
            .load(
                r#"
            local a = Vec4.new(1, 2, 3, 4)
            local b = Vec4.new(1, 2, 3, 4)
            return a == b
        "#,
            )
            .eval()
            .unwrap();
        assert!(eq);
    }

    // === Rotor4 tests ===

    #[test]
    fn test_rotor4_identity() {
        let lua = create_lua_with_math();
        let s: f32 = lua
            .load(
                r#"
            local r = Rotor4.identity()
            return r.s
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(s, 1.0);
    }

    #[test]
    fn test_rotor4_from_plane() {
        let lua = create_lua_with_math();
        // 90 degree rotation in XY plane should rotate X to Y
        let (x, y): (f32, f32) = lua
            .load(
                r#"
            local r = Rotor4.from_plane("XY", math.pi / 2)
            local v = Vec4.new(1, 0, 0, 0)
            local rotated = r:rotate(v)
            return rotated.x, rotated.y
        "#,
            )
            .eval()
            .unwrap();
        assert!(x.abs() < 0.001); // x should be ~0
        assert!((y - 1.0).abs() < 0.001); // y should be ~1
    }

    #[test]
    fn test_rotor4_composition() {
        let lua = create_lua_with_math();
        // Two 45 degree rotations should equal one 90 degree rotation
        let y: f32 = lua
            .load(
                r#"
            local r1 = Rotor4.from_plane("XY", math.pi / 4)
            local r2 = Rotor4.from_plane("XY", math.pi / 4)
            local composed = r1 * r2
            local v = Vec4.new(1, 0, 0, 0)
            local rotated = composed:rotate(v)
            return rotated.y
        "#,
            )
            .eval()
            .unwrap();
        assert!((y - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rotor4_invalid_plane() {
        let lua = create_lua_with_math();
        let result: LuaResult<()> = lua
            .load(
                r#"
            local r = Rotor4.from_plane("INVALID", 1.0)
        "#,
            )
            .exec();
        assert!(result.is_err());
    }

    // === Transform4D tests ===

    #[test]
    fn test_transform_identity() {
        let lua = create_lua_with_math();
        let x: f32 = lua
            .load(
                r#"
            local t = Transform4D.identity()
            local v = Vec4.new(1, 2, 3, 4)
            local transformed = t:transform_point(v)
            return transformed.x
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, 1.0);
    }

    #[test]
    fn test_transform_from_position() {
        let lua = create_lua_with_math();
        let x: f32 = lua
            .load(
                r#"
            local t = Transform4D.from_position(Vec4.new(10, 0, 0, 0))
            local v = Vec4.new(0, 0, 0, 0)
            local transformed = t:transform_point(v)
            return transformed.x
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, 10.0);
    }

    #[test]
    fn test_transform_direction() {
        let lua = create_lua_with_math();
        // Direction should not be affected by position
        let x: f32 = lua
            .load(
                r#"
            local t = Transform4D.from_position(Vec4.new(100, 100, 100, 100))
            local dir = Vec4.new(1, 0, 0, 0)
            local transformed = t:transform_direction(dir)
            return transformed.x
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, 1.0);
    }

    #[test]
    fn test_transform_scale() {
        let lua = create_lua_with_math();
        let x: f32 = lua
            .load(
                r#"
            local t = Transform4D.identity()
            t.scale = 2.0
            local v = Vec4.new(1, 0, 0, 0)
            local transformed = t:transform_point(v)
            return transformed.x
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, 2.0);
    }

    #[test]
    fn test_transform_field_access() {
        let lua = create_lua_with_math();
        let (x, s): (f32, f32) = lua
            .load(
                r#"
            local t = Transform4D.from_position(Vec4.new(5, 0, 0, 0))
            return t.position.x, t.scale
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(x, 5.0);
        assert_eq!(s, 1.0);
    }
}
