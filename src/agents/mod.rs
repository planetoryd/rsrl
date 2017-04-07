use {Function, Parameterised};
use domain::Transition;
use geometry::{Space, ActionSpace};
use policies::Policy;


pub trait Agent<S: Space> {
    fn act(&mut self, s: &S::Repr) -> usize;
    fn handle(&mut self, t: &Transition<S, ActionSpace>) -> usize;
}


pub mod td_zero;
pub mod actor_critic;
