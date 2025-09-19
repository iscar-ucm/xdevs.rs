use super::{DEVStoneAtomic, DEVStoneSeeder};
#[cfg(test)]
use super::{SharedProbe, TestProbe};
use crate::modeling::Coupled;

pub struct HO {
    pub coupled: Coupled,
}

impl HO {
    pub fn create(
        width: usize,
        depth: usize,
        int_delay: u64,
        ext_delay: u64,
        #[cfg(test)] probe: SharedProbe,
    ) -> Coupled {
        let mut coupled = Coupled::new("HO");
        let seeder = DEVStoneSeeder::new("seeder");
        let ho = Self::new(
            width,
            depth,
            int_delay,
            ext_delay,
            #[cfg(test)]
            probe,
        );
        let ho_name = ho.coupled.component.get_name().to_string();
        coupled.add_component(Box::new(seeder));
        coupled.add_component(Box::new(ho.coupled));
        coupled.add_ic("seeder", "output", &ho_name, "input_1");
        coupled.add_ic("seeder", "output", &ho_name, "input_2");
        coupled
    }

    fn new(
        width: usize,
        depth: usize,
        int_delay: u64,
        ext_delay: u64,
        #[cfg(test)] probe: SharedProbe,
    ) -> Self {
        // First we check the input parameters
        if width < 1 {
            panic!("width must be greater than 1")
        }
        if depth < 1 {
            panic!("depth must be greater than 1")
        }
        // Next we create the model structure
        let name = format!("coupled_{depth}");
        let mut coupled = Coupled::new(&name);
        coupled.add_in_port::<usize>("input_1");
        coupled.add_in_port::<usize>("input_2");
        coupled.add_out_port::<usize>("output_1");
        coupled.add_out_port::<usize>("output_2");
        // If this is the inner coupled model, we just add one atomic.
        if depth == 1 {
            let atomic = DEVStoneAtomic::new(
                "inner_atomic",
                int_delay,
                ext_delay,
                #[cfg(test)]
                probe.clone(),
            );
            coupled.add_component(Box::new(atomic));
            coupled.add_eic("input_1", "inner_atomic", "input");
            coupled.add_eoc("inner_atomic", "output", "output_1");
            // Otherwise, we add a subcoupled and a set of atomics.
        } else {
            let subcoupled = Self::new(
                width,
                depth - 1,
                int_delay,
                ext_delay,
                #[cfg(test)]
                probe.clone(),
            );
            let subcoupled_name = subcoupled.coupled.component.get_name().to_string();
            coupled.add_component(Box::new(subcoupled.coupled));
            coupled.add_eic("input_1", &subcoupled_name, "input_1");
            coupled.add_eic("input_1", &subcoupled_name, "input_2");
            coupled.add_eoc(&subcoupled_name, "output_1", "output_1");
            for i in 1..width {
                let atomic_name = format!("atomic_{i}");
                let atomic = DEVStoneAtomic::new(
                    &atomic_name,
                    int_delay,
                    ext_delay,
                    #[cfg(test)]
                    probe.clone(),
                );
                coupled.add_component(Box::new(atomic));
                coupled.add_eic("input_2", &atomic_name, "input");
                if i > 1 {
                    let prev_atomic_name = format!("atomic_{}", i - 1);
                    coupled.add_ic(&prev_atomic_name, "output", &atomic_name, "input");
                }
                coupled.add_eoc(&atomic_name, "output", "output_2");
            }
        }
        // Before exiting, we update the probe if required
        #[cfg(test)]
        {
            let mut x = probe.lock().unwrap();
            x.n_eics += coupled.n_eics();
            x.n_ics += coupled.n_ics();
            x.n_eocs += coupled.n_eocs();
        }
        Self { coupled }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::*;
    use std::sync::{Arc, Mutex};

    fn expected_atomics(width: usize, depth: usize) -> usize {
        (width - 1) * (depth - 1) + 1
    }

    fn expected_eics(width: usize, depth: usize) -> usize {
        (width + 1) * (depth - 1) + 1
    }

    fn expected_ics(width: usize, depth: usize) -> usize {
        match width > 2 {
            true => (width - 2) * (depth - 1),
            false => 0,
        }
    }

    fn expected_eocs(width: usize, depth: usize) -> usize {
        width * (depth - 1) + 1
    }

    fn expected_internals(width: usize, depth: usize) -> usize {
        (width - 1) * width / 2 * (depth - 1) + 1
    }

    #[test]
    fn test_ho() {
        for width in (1..50).step_by(5) {
            for depth in (1..50).step_by(5) {
                let probe = Arc::new(Mutex::new(TestProbe::default()));
                let coupled = HO::create(width, depth, 0, 0, probe.clone());
                let mut simulator = RootCoordinator::new(coupled);
                simulator.simulate(f64::INFINITY);

                let x = probe.lock().unwrap();
                assert_eq!(expected_atomics(width, depth), x.n_atomics);
                assert_eq!(expected_eics(width, depth), x.n_eics);
                assert_eq!(expected_ics(width, depth), x.n_ics);
                assert_eq!(expected_eocs(width, depth), x.n_eocs);
                assert_eq!(expected_internals(width, depth), x.n_internals);
                assert_eq!(expected_internals(width, depth), x.n_externals);
                assert_eq!(expected_internals(width, depth), x.n_events);
            }
        }
    }
}
