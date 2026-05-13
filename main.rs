use macroquad::prelude::*;
use glam::Vec2;

// ======================
// ECS BASIQUE
// ======================

type Entity = usize;

#[derive(Clone, Copy)]
struct Position(Vec2);

#[derive(Clone, Copy)]
struct Velocity(Vec2);

#[derive(Clone, Copy)]
struct Mass(f32);

#[derive(Clone, Copy)]
struct Collider {
    radius: f32,
}

// ======================
// MONDE ECS
// ======================

struct World 
{
    positions: Vec<Option<Position>>,
    velocities: Vec<Option<Velocity>>,
    masses: Vec<Option<Mass>>,
    colliders: Vec<Option<Collider>>,
}

impl World 
{
    fn new() -> Self 
    {
        Self {
            positions: vec![],
            velocities: vec![],
            masses: vec![],
            colliders: vec![],
        }
    }

    fn create_entity(&mut self) -> Entity 
    {
        let id = self.positions.len();
        self.positions.push(None);
        self.velocities.push(None);
        self.masses.push(None);
        self.colliders.push(None);
        id
    }
}

// ======================
// SYSTEMES
// ======================

// Gravité
fn gravity_system(world: &mut World, dt: f32) 
{
    let g = Vec2::new(0.0, 500.0);

    for i in 0..world.velocities.len() 
    {
        if let (Some(vel), Some(_mass)) = (world.velocities[i].as_mut(), world.masses[i]) {
            vel.0 += g * dt;
        }
    }
}

// Mouvement
fn movement_system(world: &mut World, dt: f32) 
{
    for i in 0..world.positions.len() 
    {
        // world.positions et world.velocities sont des champs distincts :
        // deux emprunts mutables de champs différents sont autorisés.
        if let (Some(pos), Some(vel)) =
            (world.positions[i].as_mut(), world.velocities[i])
        {
            pos.0 += vel.0 * dt;
        }
    }
}

// Collision sol
fn ground_collision_system(world: &mut World) 
{
    let ground = screen_height();

    for i in 0..world.positions.len() 
    {
        let col_radius = match world.colliders[i] {
            Some(c) => c.radius,
            None => continue,
        };
        // On lit d'abord, puis on modifie séparément pour rester explicite
        // vis-à-vis du borrow checker (positions et velocities = champs distincts).
        let pos_y = match world.positions[i] {
            Some(p) => p.0.y,
            None => continue,
        };
        if world.velocities[i].is_none() {
            continue;
        }

        if pos_y + col_radius > ground {
            world.positions[i].as_mut().unwrap().0.y = ground - col_radius;
            world.velocities[i].as_mut().unwrap().0.y *= -0.6; // rebond amorti
        }
    }
}

// Collision murs gauche / droite / plafond
fn wall_collision_system(world: &mut World) {
    let (w, h) = (screen_width(), screen_height());

    for i in 0..world.positions.len() {
        let col_radius = match world.colliders[i] {
            Some(c) => c.radius,
            None => continue,
        };
        let pos = match world.positions[i] {
            Some(p) => p.0,
            None => continue,
        };
        if world.velocities[i].is_none() {
            continue;
        }

        let p = world.positions[i].as_mut().unwrap();
        let v = world.velocities[i].as_mut().unwrap();

        // Mur gauche
        if pos.x - col_radius < 0.0 {
            p.0.x = col_radius;
            v.0.x *= -0.6;
        }
        // Mur droit
        if pos.x + col_radius > w {
            p.0.x = w - col_radius;
            v.0.x *= -0.6;
        }
        // Plafond
        if pos.y - col_radius < 0.0 {
            p.0.y = col_radius;
            v.0.y *= -0.6;
        }
        // Sol (déjà géré par ground_collision_system, garde-fou ici)
        if pos.y + col_radius > h {
            p.0.y = h - col_radius;
            v.0.y *= -0.6;
        }
    }
}

// Collision entre sphères
//
// CORRECTION : on ne peut pas emprunter world.velocities[i] et
// world.velocities[j] comme deux &mut depuis le même Vec simultanément.
// Solution : collecter toutes les collisions d'abord (lecture seule),
// puis appliquer les corrections (écriture) dans une seconde passe.
fn collision_system(world: &mut World) {
    let len = world.positions.len();

    // --- Passe 1 : détection (lecture seule) ---
    // On stocke (i, j, normale, overlap) pour chaque paire en collision.
    let mut collisions: Vec<(usize, usize, Vec2, f32)> = vec![];

    for i in 0..len {
        for j in (i + 1)..len {
            let (pos_a, pos_b, col_a, col_b) = match (
                world.positions[i],
                world.positions[j],
                world.colliders[i],
                world.colliders[j],
            ) {
                (Some(a), Some(b), Some(ca), Some(cb)) => (a, b, ca, cb),
                _ => continue,
            };

            if world.velocities[i].is_none() || world.velocities[j].is_none() {
                continue;
            }

            let delta = pos_b.0 - pos_a.0;
            let dist = delta.length();
            let min_dist = col_a.radius + col_b.radius;

            if dist < min_dist && dist > 0.0 {
                let normal = delta / dist;
                let overlap = min_dist - dist;
                collisions.push((i, j, normal, overlap));
            }
        }
    }

    // --- Passe 2 : résolution (écriture) ---
    for (i, j, normal, overlap) in collisions {
        // Séparation positionnelle
        if let Some(p) = world.positions[i].as_mut() {
            p.0 -= normal * overlap * 0.5;
        }
        if let Some(p) = world.positions[j].as_mut() {
            p.0 += normal * overlap * 0.5;
        }

        // Échange de vitesse : on copie d'abord les valeurs (Copy),
        // puis on les réassigne séparément — pas de double &mut sur le même Vec.
        let vi = world.velocities[i].map(|v| v.0);
        let vj = world.velocities[j].map(|v| v.0);

        if let (Some(vi), Some(vj)) = (vi, vj) {
            if let Some(v) = world.velocities[i].as_mut() {
                v.0 = vj * 0.8;
            }
            if let Some(v) = world.velocities[j].as_mut() {
                v.0 = vi * 0.8;
            }
        }
    }
}

// ======================
// RENDU
// ======================

fn render_system(world: &World) {
    // Ligne de sol
    draw_line(
        0.0,
        screen_height(),
        screen_width(),
        screen_height(),
        2.0,
        GRAY,
    );

    // Contour de l'arène
    draw_rectangle_lines(0.0, 0.0, screen_width(), screen_height(), 2.0, DARKGRAY);

    for i in 0..world.positions.len() {
        if let (Some(pos), Some(col)) = (world.positions[i], world.colliders[i]) {
            // Corps de la balle (bleu translucide)
            draw_circle(
                pos.0.x,
                pos.0.y,
                col.radius,
                Color::from_rgba(80, 160, 255, 200),
            );
            // Contour blanc pour la lisibilité
            draw_circle_lines(pos.0.x, pos.0.y, col.radius, 1.5, WHITE);
        }
    }
}

// ======================
// MAIN LOOP
// ======================

#[macroquad::main("Physics ECS")]
async fn main() {
    let mut world = World::new();

    // Création des entités
    for i in 0..10 {
        let e = world.create_entity();

        world.positions[e] = Some(Position(Vec2::new(200.0 + i as f32 * 40.0, 100.0)));

        world.velocities[e] = Some(Velocity(Vec2::new(rand::gen_range(-50.0, 50.0), 0.0)));

        world.masses[e] = Some(Mass(1.0));

        world.colliders[e] = Some(Collider { radius: 15.0 });
    }

    loop {
        // On plafonne dt à ~33 ms pour éviter les explosions numériques
        // lors d'un pic de frame (chargement, alt-tab, etc.).
        let dt = get_frame_time().min(0.033);

        clear_background(Color::from_rgba(10, 10, 20, 255));

        // === SYSTEMES ===
        gravity_system(&mut world, dt);
        movement_system(&mut world, dt);
        collision_system(&mut world);
        ground_collision_system(&mut world);
        wall_collision_system(&mut world); // nouveau : maintient les balles dans l'écran

        // === RENDU ===
        render_system(&world);

        // Compteur FPS (coin haut-gauche)
        draw_text(
            &format!("FPS: {}", get_fps()),
            10.0,
            22.0,
            20.0,
            Color::from_rgba(180, 180, 180, 200),
        );

        next_frame().await;
    }
}