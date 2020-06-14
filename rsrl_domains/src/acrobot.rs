use super::{runge_kutta4, Domain, Observation, Reward};
use crate::{
    consts::{G, PI_OVER_2},
    spaces::{discrete::Ordinal, real::Interval, ProductSpace},
};
use std::f64::consts::PI;

// Link masses:
const M1: f64 = 1.0;
const M2: f64 = 1.0;

// Link lengths:
const L1: f64 = 1.0;
#[allow(dead_code)]
const L2: f64 = 1.0;

// Link centre of masses:
const LC1: f64 = 0.5;
const LC2: f64 = 0.5;

// Link moment of intertias:
const I1: f64 = 1.0;
const I2: f64 = 1.0;

const DT: f64 = 0.2;

const LIMITS_THETA1: [f64; 2] = [-PI, PI];
const LIMITS_THETA2: [f64; 2] = [-PI, PI];
const LIMITS_DTHETA1: [f64; 2] = [-4.0 * PI, 4.0 * PI];
const LIMITS_DTHETA2: [f64; 2] = [-9.0 * PI, 9.0 * PI];

const REWARD_STEP: f64 = -1.0;
const REWARD_TERMINAL: f64 = 0.0;

const TORQUE: f64 = 1.0;
const ALL_ACTIONS: [f64; 3] = [-TORQUE, 0.0, TORQUE];

make_index!(StateIndex [
    THETA1 => 0, THETA2 => 1, DTHETA1 => 2, DTHETA2 => 3
]);

/// Classic double pendulum control domain.
///
/// The acrobot is a 2-link pendulum environment in which only the second joint
/// actuated. The goal is to swing the end-effector to a distance equal to the
/// length of one link above the base.
///
/// See [https://www.math24.net/double-pendulum/](https://www.math24.net/double-pendulum/)
pub struct Acrobot([f64; 4]);

impl Acrobot {
    pub fn new(theta1: f64, theta2: f64, dtheta1: f64, dtheta2: f64) -> Acrobot {
        Acrobot([theta1, theta2, dtheta1, dtheta2])
    }

    fn is_terminal(theta1: f64, theta2: f64) -> bool {
        theta1.cos() + (theta1 + theta2).cos() < -1.0
    }

    fn update_state(&mut self, a: usize) {
        let fx = |_x, y| Acrobot::grad(ALL_ACTIONS[a], y);
        let ns = runge_kutta4(&fx, 0.0, self.0.to_vec(), DT);

        self.0[StateIndex::THETA1] =
            wrap!(LIMITS_THETA1[0], ns[StateIndex::THETA1], LIMITS_THETA1[1]);
        self.0[StateIndex::THETA2] =
            wrap!(LIMITS_THETA2[0], ns[StateIndex::THETA2], LIMITS_THETA2[1]);

        self.0[StateIndex::DTHETA1] = clip!(
            LIMITS_DTHETA1[0],
            ns[StateIndex::DTHETA1],
            LIMITS_DTHETA1[1]
        );
        self.0[StateIndex::DTHETA2] = clip!(
            LIMITS_DTHETA2[0],
            ns[StateIndex::DTHETA2],
            LIMITS_DTHETA2[1]
        );
    }

    fn grad(torque: f64, mut buffer: Vec<f64>) -> Vec<f64> {
        let theta1 = buffer[StateIndex::THETA1];
        let theta2 = buffer[StateIndex::THETA2];
        let dtheta1 = buffer[StateIndex::DTHETA1];
        let dtheta2 = buffer[StateIndex::DTHETA2];

        buffer[StateIndex::THETA1] = dtheta1;
        buffer[StateIndex::THETA2] = dtheta2;

        let sin_t2 = theta2.sin();
        let cos_t2 = theta2.cos();

        let d1 = M1 * LC1 * LC1 + M2 * (L1 * L1 + LC2 * LC2 + 2.0 * L1 * LC2 * cos_t2) + I1 + I2;
        let d2 = M2 * (LC2 * LC2 + L1 * LC2 * cos_t2) + I2;

        let phi2 = M2 * LC2 * G * (theta1 + theta2 - PI_OVER_2).cos();
        let phi1 = -1.0 * L1 * LC2 * dtheta2 * dtheta2 * sin_t2
            - 2.0 * M2 * L1 * LC2 * dtheta2 * dtheta1 * sin_t2
            + (M1 * LC1 + M2 * L1) * G * (theta1 - PI_OVER_2).cos()
            + phi2;

        buffer[StateIndex::DTHETA1] =
            (torque + d2 / d1 * phi1 - M2 * L1 * LC2 * dtheta1 * dtheta1 * sin_t2 - phi2)
                / (M2 * LC2 * LC2 + I2 - d2 * d2 / d1);
        buffer[StateIndex::DTHETA2] = -(d2 * buffer[StateIndex::DTHETA1] + phi1) / d1;

        buffer
    }
}

impl Default for Acrobot {
    fn default() -> Acrobot { Acrobot::new(0.0, 0.0, 0.0, 0.0) }
}

impl Domain for Acrobot {
    type StateSpace = ProductSpace<Interval>;
    type ActionSpace = Ordinal;

    fn emit(&self) -> Observation<Vec<f64>> {
        let theta1 = self.0[StateIndex::THETA1];
        let theta2 = self.0[StateIndex::THETA2];

        if Acrobot::is_terminal(theta1, theta2) {
            Observation::Terminal(self.0.to_vec())
        } else {
            Observation::Full(self.0.to_vec())
        }
    }

    fn step(&mut self, action: &usize) -> (Observation<Vec<f64>>, Reward) {
        self.update_state(*action);

        let to = self.emit();
        let reward = if to.is_terminal() {
            REWARD_TERMINAL
        } else {
            REWARD_STEP
        };

        (to, reward)
    }

    fn state_space(&self) -> Self::StateSpace {
        ProductSpace::empty()
            + Interval::bounded(LIMITS_THETA1[0], LIMITS_THETA1[1])
            + Interval::bounded(LIMITS_THETA2[0], LIMITS_THETA2[1])
            + Interval::bounded(LIMITS_DTHETA1[0], LIMITS_DTHETA1[1])
            + Interval::bounded(LIMITS_DTHETA2[0], LIMITS_DTHETA2[1])
    }

    fn action_space(&self) -> Ordinal { Ordinal::new(3) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Domain, Observation};

    #[test]
    fn test_initial_observation() {
        let m = Acrobot::default();

        match m.emit() {
            Observation::Full(ref state) => {
                assert_eq!(state[0], 0.0);
                assert_eq!(state[1], 0.0);
                assert_eq!(state[2], 0.0);
                assert_eq!(state[3], 0.0);
            },
            _ => panic!("Should yield a fully observable state."),
        }

        assert!(!m.emit().is_terminal());
    }
}
