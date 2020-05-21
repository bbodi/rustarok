use rustarok_common::common::GameTime;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct SimulationTime {
    pub render_frame: u64,
    skip_next_simulation: bool,
    run_simulation_in_this_frame: bool,
    last_simulation_at: Instant,
    time_between_simulations: Duration,
}

impl SimulationTime {
    pub fn new(simulation_freq: usize) -> SimulationTime {
        SimulationTime {
            render_frame: 1,
            skip_next_simulation: false,
            run_simulation_in_this_frame: true,
            last_simulation_at: Instant::now(),

            time_between_simulations: Duration::from_millis((1000 / simulation_freq) as u64),
        }
    }

    #[cfg(test)]
    pub fn new_for_tests(fix_dt_for_test: Duration) -> SimulationTime {
        SimulationTime {
            last_simulation_at: Instant::now(),
            render_frame: 1,
            run_simulation_in_this_frame: true,
            skip_next_simulation: false,
            time_between_simulations: Duration::from_millis(30 as u64),
        }
    }

    pub fn can_simulation_run(&self) -> bool {
        self.run_simulation_in_this_frame
    }

    pub fn get_time_between_simulations(&self) -> Duration {
        self.time_between_simulations
    }

    pub fn adjust_simulation_freq(&mut self, simulation_duration_adjuster: i64) {
        if simulation_duration_adjuster > 0 {
            self.time_between_simulations +=
                Duration::from_millis(simulation_duration_adjuster as u64);
        } else if simulation_duration_adjuster < 0 {
            let tmp = simulation_duration_adjuster.abs();
            if self.time_between_simulations.as_millis() as i64 > tmp {
                self.time_between_simulations -= Duration::from_millis(tmp as u64);
            }
        }
    }

    pub fn render_frame_end(&mut self, now: Instant) {
        self.render_frame += 1;

        self.run_simulation_in_this_frame =
            self.last_simulation_at.elapsed() >= self.time_between_simulations;
        if self.run_simulation_in_this_frame {
            if self.skip_next_simulation {
                self.run_simulation_in_this_frame = false;
                self.skip_next_simulation = false;
            } else {
                self.last_simulation_at = now;
            }
        }
    }

    pub fn force_simulation(&mut self) {
        self.run_simulation_in_this_frame = true;
    }

    pub fn skip_next_simulation(&mut self) {
        self.skip_next_simulation = true;
    }
}
