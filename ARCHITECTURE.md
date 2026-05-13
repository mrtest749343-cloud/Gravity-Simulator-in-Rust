# 🏗️ Gravity Simulator - Component Architecture Breakdown

## Overview: From Problem to Code

**The Problem:** Simulate realistic physics with bouncing spheres in real-time

**The Solution:** Separate concerns into modular components that work together

---

## 📦 Part 1: Core Data Structures (Components)

These are the **fundamental building blocks** - each represents ONE piece of information about a physics object.

### 1️⃣ **Position Component** 
```rust
#[derive(Clone, Copy)]
struct Position(Vec2);
```
- **What it is:** The (x, y) location of a sphere in the world
- **Physics Concept:** State variable - where is the object?
- **Problem it solves:** "Where should I render this sphere?"
- **Immutable until:** Movement system updates it

---

### 2️⃣ **Velocity Component**
```rust
#[derive(Clone, Copy)]
struct Velocity(Vec2);
```
- **What it is:** The (vx, vy) speed and direction of movement
- **Physics Concept:** First derivative of position - how fast is it moving?
- **Problem it solves:** "How much should position change each frame?"
- **Modified by:** Gravity system, collision system, wall collisions

---

### 3️⃣ **Mass Component**
```rust
#[derive(Clone, Copy)]
struct Mass(f32);
```
- **What it is:** The weight/inertia of the sphere
- **Physics Concept:** Resistance to acceleration
- **Problem it solves:** "Should this object accelerate equally with gravity?"
- **Status:** Currently not used (reserved for future physics)
- **Note:** Could be used for f = m*a calculations

---

### 4️⃣ **Collider Component**
```rust
#[derive(Clone, Copy)]
struct Collider {
    radius: f32,
}
```
- **What it is:** Physical shape and size (sphere radius)
- **Physics Concept:** Boundary definition
- **Problem it solves:** "Does this sphere touch that sphere?"
- **Used by:** All collision detection systems

---

## 🔄 Part 2: The World Container (ECS Data Store)

```rust
struct World {
    positions: Vec<Option<Position>>,        // 🎯 WHERE
    velocities: Vec<Option<Velocity>>,       // 🎯 HOW FAST
    masses: Vec<Option<Mass>>,               // 🎯 HOW HEAVY
    colliders: Vec<Option<Collider>>,        // 🎯 WHAT SHAPE
}
```

### Why `Vec<Option<T>>`?
- **Sparse Storage:** Not all entities have all components (flexibility)
- **Safe Access:** `Option` forces you to handle missing data
- **Fast Indexing:** `Vec[i]` is O(1), perfect for real-time

### The Entity ID Pattern
```rust
type Entity = usize;

// Entity 0 has Position[0], Velocity[0], Mass[0], Collider[0]
// Entity 1 has Position[1], Velocity[1], Mass[1], Collider[1]
// etc...
```

---

## ⚙️ Part 3: Physics Systems (Behavior Layer)

Each system is a **pure function** that operates on the world. They transform state.

### System 1️⃣: **Gravity System** 
```
📥 Input:  World with velocities
📊 Logic:  velocity += gravity * dt
📤 Output: Updated velocities
```

**Physics it implements:**
- Newton's 2nd Law: F = ma
- Free fall: v = v₀ + gt
- g = 500 pixels/s² (arbitrary down direction)

**Problem solved:** "How do we make things fall?"

**Code pattern:**
```rust
fn gravity_system(world: &mut World, dt: f32) {
    let g = Vec2::new(0.0, 500.0);  // Gravity vector
    for i in 0..world.velocities.len() {
        if let Some(vel) = world.velocities[i].as_mut() {
            vel.0 += g * dt;  // Integrate acceleration
        }
    }
}
```

---

### System 2️⃣: **Movement System**
```
📥 Input:  World with positions & velocities
📊 Logic:  position += velocity * dt
📤 Output: Updated positions
```

**Physics it implements:**
- Kinematics: x = x₀ + v*t
- Basic integration (Euler method - simple but works)

**Problem solved:** "How do we update positions based on velocity?"

**Code pattern:**
```rust
fn movement_system(world: &mut World, dt: f32) {
    for i in 0..world.positions.len() {
        if let (Some(pos), Some(vel)) = 
            (world.positions[i].as_mut(), world.velocities[i])
        {
            pos.0 += vel.0 * dt;  // Euler integration
        }
    }
}
```

---

### System 3️⃣: **Collision System (Sphere-to-Sphere)**
```
📥 Input:  World with positions, velocities, colliders
📊 Logic:  Detect overlaps → Separate → Exchange velocities
📤 Output: Updated positions & velocities
```

**Physics it implements:**
- Distance formula: dist = √((x₂-x₁)² + (y₂-y₁)²)
- Collision detection: dist < (r₁ + r₂)?
- Collision resolution: Swap velocities with dampening
- Positional correction: Separate overlapping spheres

**Problem solved:** "How do we handle sphere collisions?"

**Two-Pass Architecture:**

```
Pass 1: DETECT (Read-Only)
├─ For each pair (i, j) where i < j:
│  ├─ Read position[i], position[j]
│  ├─ Read collider[i], collider[j]
│  ├─ Calculate distance
│  └─ If overlapping: store (i, j, normal, overlap)
└─ Result: List of collisions

Pass 2: RESOLVE (Write)
└─ For each collision in list:
   ├─ Separate positions using normal vector
   ├─ Copy velocities (because Copy trait!)
   └─ Swap and dampen velocities
```

**Why two passes?**
- Rust's borrow checker won't allow `&mut velocity[i]` and `&mut velocity[j]` simultaneously
- Solution: Collect data first (immutable), then modify (mutable)

**Code complexity:**
```rust
fn collision_system(world: &mut World) {
    let len = world.positions.len();
    let mut collisions = vec![];  // Collect first
    
    // Pass 1: Detect
    for i in 0..len {
        for j in (i+1)..len {
            if let (Some(a), Some(b), Some(ca), Some(cb)) = 
                (world.positions[i], world.positions[j],
                 world.colliders[i], world.colliders[j])
            {
                let delta = b.0 - a.0;
                let dist = delta.length();
                let min_dist = ca.radius + cb.radius;
                
                if dist < min_dist && dist > 0.0 {
                    let normal = delta / dist;
                    let overlap = min_dist - dist;
                    collisions.push((i, j, normal, overlap));
                }
            }
        }
    }
    
    // Pass 2: Resolve
    for (i, j, normal, overlap) in collisions {
        // Separate
        if let Some(p) = world.positions[i].as_mut() {
            p.0 -= normal * overlap * 0.5;
        }
        if let Some(p) = world.positions[j].as_mut() {
            p.0 += normal * overlap * 0.5;
        }
        
        // Swap velocities
        let vi = world.velocities[i].map(|v| v.0);
        let vj = world.velocities[j].map(|v| v.0);
        if let (Some(vi), Some(vj)) = (vi, vj) {
            world.velocities[i].as_mut().unwrap().0 = vj * 0.8;
            world.velocities[j].as_mut().unwrap().0 = vi * 0.8;
        }
    }
}
```

---

### System 4️⃣: **Ground Collision System**
```
📥 Input:  World with positions, velocities, colliders
📊 Logic:  if position.y + radius > screen_height:
           - Clamp position
           - Reverse & dampen velocity
📤 Output: Updated positions & velocities
```

**Physics it implements:**
- Boundary collision (one-sided)
- Coefficient of restitution: 0.6 (60% bounce back)

**Problem solved:** "How do we keep spheres from falling off the bottom?"

---

### System 5️⃣: **Wall Collision System**
```
📥 Input:  World with positions, velocities, colliders
📊 Logic:  Check all 4 boundaries (left, right, top, bottom)
📤 Output: Updated positions & velocities
```

**Physics it implements:**
- Boundary collision (all 4 sides + ceiling)
- Same dampening (0.6)

**Problem solved:** "How do we keep spheres from escaping the box?"

---

### System 6️⃣: **Render System** (Not Physics, But Critical!)
```
📥 Input:  World with positions, colliders
📊 Logic:  For each entity: draw_circle(pos, radius)
📤 Output: Screen pixels (visualization)
```

**Problem solved:** "How do we see the simulation?"

---

## 📊 Part 4: Data Flow Diagram

```
┌──────────────────────────────────────────────┐
│         FRAME LOOP (60x per second)          │
└──────────────────────────────────────────────┘
                        │
                        ⬇️
    ┌───────────────────────────────────────┐
    │  1. Gravity System                     │
    │  ─────────────────────────────────   │
    │  Input:  velocities                   │
    │  Output: velocities += gravity * dt   │
    │  Result: Things accelerate downward   │
    └───────────────────────────────────────┘
                        │
                        ⬇️
    ┌───────────────────────────────────────┐
    │  2. Movement System                    │
    │  ─────────────────────────────────   │
    │  Input:  positions, velocities        │
    │  Output: positions += velocities * dt │
    │  Result: Things move                  │
    └───────────────────────────────────────┘
                        │
                        ⬇️
    ┌───────────────────────────────────────┐
    │  3. Collision System (Sphere-Sphere)  │
    │  ─────────────────────────────────   │
    │  Input:  positions, velocities        │
    │  Output: collisions resolved          │
    │  Result: Things bounce off each other │
    └───────────────────────────────────────┘
                        │
                        ⬇️
    ┌───────────────────────────────────────┐
    │  4. Ground Collision System            │
    │  ─────────────────────────────────   │
    │  Input:  positions, velocities        │
    │  Output: positions, velocities        │
    │  Result: Bounce off floor            │
    └───────────────────────────────────────┘
                        │
                        ⬇️
    ┌───────────────────────────────────────┐
    │  5. Wall Collision System              │
    │  ─────────────────────────────────   │
    │  Input:  positions, velocities        │
    │  Output: positions, velocities        │
    │  Result: Bounce off walls/ceiling     │
    └───────────────────────────────────────┘
                        │
                        ⬇️
    ┌───────────────────────────────────────┐
    │  6. Render System                      │
    │  ─────────────────────────────────   │
    │  Input:  positions, colliders         │
    │  Output: pixels on screen             │
    │  Result: We see the spheres!          │
    └───────────────────────────────────────┘
```

---

## 🎯 Part 5: Component-to-Physics Mapping

| Component | Represents | Used By | Physics Equation |
|-----------|-----------|---------|-----------------|
| **Position** | Current location (x, y) | Render, Collisions | x(t) = x₀ + v₀*t + ½*a*t² |
| **Velocity** | Speed & direction (vx, vy) | Movement, Gravity, Collisions | v(t) = v₀ + a*t |
| **Mass** | Inertia (unused) | Could be gravity | F = m*a |
| **Collider** | Physical shape (radius) | All collision systems | r = collision_radius |

---

## 🧩 Part 6: Conceptual Mapping - Problem → Solution

### Problem 1: "Objects fall"
- **Component needed:** Velocity (what accelerates)
- **System needed:** Gravity (applies acceleration)
- **Equation:** v += g * dt

### Problem 2: "Objects move"
- **Component needed:** Position (what changes), Velocity (how much)
- **System needed:** Movement (integrates velocity)
- **Equation:** p += v * dt

### Problem 3: "Objects collide (sphere-to-sphere)"
- **Component needed:** Position (where), Collider (size), Velocity (momentum)
- **System needed:** Collision detection + resolution
- **Equations:**
  - dist = length(pos₂ - pos₁)
  - if dist < r₁ + r₂: collision!
  - v'₁ = v₂ * dampen, v'₂ = v₁ * dampen

### Problem 4: "Objects bounce off boundaries"
- **Component needed:** Position (boundary check), Velocity (bounce), Collider (size)
- **System needed:** Ground & Wall collision systems
- **Logic:** if pos.y + r > screen_height: reverse velocity & clamp position

### Problem 5: "We see something"
- **Component needed:** Position (where to draw), Collider (how big)
- **System needed:** Render
- **Function:** draw_circle(pos, radius)

---

## 🔗 Part 7: Dependency Graph

```
Gravity System
    ↓ modifies
Velocity
    ↓ used by
Movement System
    ↓ modifies
Position
    ↓ used by
Collision System ←── also uses ──→ Collider
    ↓ modifies
Velocity (back to gravity next frame!)
    ↓ used by
Render System
    ↓ displays
Screen

Wall/Ground Collision Systems also modify (Position, Velocity)
```

---

## 💡 Part 8: Why This Structure Works

### ✅ **Separation of Concerns**
- Each system handles ONE responsibility
- Gravity doesn't know about collisions
- Collisions don't know about rendering
- Easy to test, debug, modify

### ✅ **Data-Oriented Design**
- Components are POD (Plain Old Data) - simple structs
- Systems operate on data, not objects
- Cache-friendly (vectors are contiguous)
- Fast iteration

### ✅ **Real-Time Performance**
- No allocations per frame
- `Copy` types = no heap allocation
- O(n) gravity, O(n) movement, O(n²) collision (acceptable for n=10)

### ✅ **Rust Safety**
- Borrow checker prevents bugs
- No null pointers (use `Option`)
- No data races (single-threaded for now)

---

## 📝 Part 9: Extension Ideas (Future Systems)

| New System | What it does | Component needed |
|-----------|-------------|------------------|
| **Friction** | Slow objects down on ground | Friction coefficient |
| **Magnetic Force** | Attract/repel objects | Charge component |
| **Air Resistance** | Drag force opposing motion | Drag coefficient |
| **Rotation** | Spinning objects | Rotation, Angular velocity |
| **Soft Bodies** | Deformable spheres | Deformation state |
| **Fluid Dynamics** | Water/air simulation | Density, viscosity |

---

## 🎓 Conclusion: The Elegant Pattern

```
PROBLEM → COMPONENTS → SYSTEMS → SOLUTION

Physics Simulation → (Position, Velocity, Collider) → (Gravity, Movement, Collision) → Bouncing Spheres
```

Each piece is:
- **Minimal:** Does one thing well
- **Reusable:** Systems can be combined
- **Testable:** Each system is a pure function
- **Efficient:** Data-oriented, cache-friendly
- **Extensible:** Add new systems without breaking existing ones

This is **professional game engine architecture**, applied to a simple physics simulation. 🚀

---

**Now you can see the grand design!** Each component, each system, each line of code serves a specific purpose in the larger ecosystem. 👨‍💻
