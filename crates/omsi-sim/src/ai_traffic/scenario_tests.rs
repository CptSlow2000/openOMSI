//! Whole traffic scenes run headless, for the ways traffic used to lock up: four cars at
//! a junction without priorities, a queue beyond a junction, a side road at a busy main
//! road, a queue moving off at a green light.

use super::*;
use crate::ai_traffic::setup::RandomTypes;
use crate::traffic::{LaneBuilder, LaneKey};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A vehicle type with no model and no scripts, in a folder of its own.
struct Fixture {
    dir: std::path::PathBuf,
    ty: Arc<VehicleType>,
}

impl Fixture {
    fn new() -> Fixture {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "omsi-traffic-scenario-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("car.bus"), "[model]\nmodel.cfg\n").unwrap();
        std::fs::write(dir.join("model.cfg"), "").unwrap();
        let ty = Arc::new(VehicleType::load_ai(&dir, &dir.join("car.bus")).unwrap());
        Fixture { dir, ty }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn traffic(f: &Fixture, mut net: Network) -> TrafficSim {
    net.build_grid();
    net.compute_reach();
    let random = RandomTypes { types: Vec::new(), groups: Vec::new(), group_curves: false, group_uvg: Vec::new(), uvg_defaults: Vec::new() };
    TrafficSim::assemble(&f.dir, net, random, Vec::new(), HashMap::new(), (Vec::new(), Vec::new()), Vec::new(), (1.0, 0), 0)
}

fn add_car(t: &mut TrafficSim, f: &Fixture, lane: usize, s: f32, seed: u64, speed: Option<f32>) -> u64 {
    let vehicle = VehicleInstance::new(f.ty.clone(), crate::VehicleHost::new(crate::SimClock::default()));
    t.place_car(vehicle, LaneKind::Street, lane, s, f.ty.clone(), seed, None, None, speed, None)
}

fn street(start: DVec3, heading: f64, length: f64) -> crate::traffic::Lane {
    LaneBuilder::arc(start, heading, length, 0.0, 0.0, LaneKind::Street, 3.0)
}

/// A junction object (one key, a path per direction) where four straight roads meet,
/// 150 m of road before each of its paths and `exit` m after: lanes 0..4 the roads in
/// (from the south, west, north, east), 4..8 the junction's paths, 8..12 the roads out.
/// Without priorities: everybody gives way to the right.
fn crossroads(exit: f64) -> Network {
    let half = 12.0;
    let o = 1.75;
    // (start of the junction path, heading) north, east, south, west bound
    let dirs = [
        (DVec3::new(o, -half, 0.0), 0.0),
        (DVec3::new(-half, -o, 0.0), 90.0),
        (DVec3::new(-o, half, 0.0), 180.0),
        (DVec3::new(half, o, 0.0), 270.0),
    ];
    let back = |p: DVec3, h: f64, d: f64| {
        let r = h.to_radians();
        p - DVec3::new(r.sin() * d, r.cos() * d, 0.0)
    };
    let mut lanes = Vec::new();
    for &(p, h) in &dirs {
        lanes.push(street(back(p, h, 150.0), h, 150.0));
    }
    for (k, &(p, h)) in dirs.iter().enumerate() {
        let mut l = street(p, h, 2.0 * half);
        l.source = 2;
        l.key = Some(LaneKey { tile: (0, 0), id: 1, path: k as u16 });
        lanes.push(l);
    }
    for k in 0..4 {
        let end = lanes[4 + k].end();
        lanes.push(street(end, dirs[k].1, exit));
    }
    let mut net = Network { lanes, ..Default::default() };
    net.link(1.5);
    net
}

fn run(t: &mut TrafficSim, secs: f32) {
    for _ in 0..(secs / 0.05) as usize {
        t.tick(0.05, None);
    }
}

/// The car's front, metres past the junction's middle along its own road (+ = through).
fn past_middle(t: &TrafficSim, id: u64) -> Option<f64> {
    let c = t.cars.iter().find(|c| c.id == id)?;
    let h = c.vehicle.heading.to_radians();
    let fwd = DVec2::new(h.sin(), h.cos());
    Some(c.vehicle.position.truncate().dot(fwd) + c.state.front as f64)
}

#[test]
fn four_cars_at_a_junction_without_priorities_all_get_through() {
    // everybody has somebody on the right: by the rules alone nobody would ever go
    let f = Fixture::new();
    let mut t = traffic(&f, crossroads(150.0));
    let ids: Vec<u64> = (0..4).map(|k| add_car(&mut t, &f, k, 135.0, 0x51 + k as u64 * 0x1000, Some(4.0))).collect();
    run(&mut t, 40.0);
    for id in ids {
        if let Some(p) = past_middle(&t, id) {
            assert!(p > 15.0, "car {id} has got only {p:.1} m past the junction's middle");
        }
    }
}

#[test]
fn a_car_does_not_drive_into_a_junction_it_cannot_leave() {
    // a queue crawls along the road beyond the junction with no room for another car:
    // the car coming up stops at the junction's line, not in its middle
    let f = Fixture::new();
    let mut t = traffic(&f, crossroads(14.0));
    // (the road out ends in 14 m: the queue on it stands at its end)
    let lane_out = 8;
    let front = add_car(&mut t, &f, lane_out, 10.0, 0x77, Some(0.0));
    let second = add_car(&mut t, &f, lane_out, 3.0, 0x78, Some(0.0));
    // the car that comes up from the south
    let me = add_car(&mut t, &f, 0, 90.0, 0x79, Some(8.0));
    for _ in 0..(30.0 / 0.05) as usize {
        // the two ahead stay where they are (a queue that does not move on)
        for c in t.cars.iter_mut().filter(|c| c.id == front || c.id == second) {
            c.state.speed = 0.0;
            c.state.max_speed_kmh = 0.0;
        }
        t.tick(0.05, None);
    }
    let c = t.cars.iter().find(|c| c.id == me).expect("still there");
    assert!(c.state.lane == 0, "the car drove into the junction (lane {} s {:.1})", c.state.lane, c.state.s);
    assert!(c.state.speed < 0.1, "it stands at the line ({:.1} m/s)", c.state.speed);
}

#[test]
fn the_exit_counts_a_crawling_queue_where_it_will_stop() {
    let way = [(10, -20.0), (20, 5.0), (21, 15.0)];
    // the last car of the queue 4 m beyond the exit's start, crawling at 1.8 m/s (it will
    // be 0.8 m further on when it has stopped): no room for 7 m
    assert!(queued_exit_vehicle(&way, (20, 5.0), 7.0, [(20, 4.0, 1.8)]).is_some());
    // driving off at 8 m/s: room
    assert!(queued_exit_vehicle(&way, (20, 5.0), 7.0, [(20, 4.0, 8.0)]).is_none());
}

#[test]
fn a_queue_moves_off_without_closing_up_or_braking_hard() {
    // ten cars standing nose to tail move off together (the Intelligent Driver Model):
    // nobody runs into the one ahead, nobody brakes hard, and the queue is on its way
    let f = Fixture::new();
    let mut lanes = vec![street(DVec3::ZERO, 0.0, 900.0)];
    lanes[0].speed_limit_kmh = 50.0;
    let mut net = Network { lanes, ..Default::default() };
    net.link(1.5);
    let mut t = traffic(&f, net);
    let ids: Vec<u64> = (0..10).map(|k| add_car(&mut t, &f, 0, 200.0 - k as f32 * 7.0, 0x300 + k as u64 * 0x77, Some(0.0))).collect();
    let mut min_gap = f32::MAX;
    let mut hardest = 0.0f32;
    for _ in 0..(40.0 / 0.05) as usize {
        t.tick(0.05, None);
        for w in ids.windows(2) {
            let (Some(a), Some(b)) = (t.cars.iter().find(|c| c.id == w[0]), t.cars.iter().find(|c| c.id == w[1])) else { continue };
            min_gap = min_gap.min(a.state.s - a.state.rear - (b.state.s + b.state.front));
        }
        hardest = hardest.min(t.cars.iter().map(|c| c.state.acc).fold(0.0, f32::min));
    }
    assert!(min_gap > 1.0, "two cars came within {min_gap:.2} m");
    assert!(hardest > -3.0, "a car braked at {hardest:.1} m/s²");
    for id in ids {
        let c = t.cars.iter().find(|c| c.id == id).unwrap();
        assert!(c.state.speed > 8.0, "car {id} at {:.1} m/s after 40 s", c.state.speed);
    }
}
