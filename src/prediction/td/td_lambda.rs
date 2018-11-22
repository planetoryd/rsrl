use core::{Algorithm, Predictor, Parameter, Trace};
use domains::Transition;
use fa::{Approximator, Parameterised, Projection, Projector, SimpleLFA, VFunction};
use geometry::Matrix;

pub struct TDLambda<S: ?Sized, P: Projector<S>> {
    trace: Trace,

    pub fa_theta: SimpleLFA<S, P>,

    pub alpha: Parameter,
    pub gamma: Parameter,
}

impl<S: ?Sized, P: Projector<S>> TDLambda<S, P> {
    pub fn new<T1, T2>(trace: Trace, fa_theta: SimpleLFA<S, P>, alpha: T1, gamma: T2) -> Self
    where
        T1: Into<Parameter>,
        T2: Into<Parameter>,
    {
        TDLambda {
            trace,

            fa_theta,

            alpha: alpha.into(),
            gamma: gamma.into(),
        }
    }

    #[inline(always)]
    fn update_v(&mut self, phi_s: Projection, error: f64) {
        let decay_rate = self.trace.lambda.value() * self.gamma.value();

        self.trace.decay(decay_rate);
        self.trace
            .update(&phi_s.expanded(self.fa_theta.projector.dim()));

        self.fa_theta
            .update_phi(&Projection::Dense(self.trace.get()), self.alpha * error);
    }
}

impl<S, A, M: Projector<S>> Algorithm<S, A> for TDLambda<S, M>
where
    Self: Predictor<S, A>
{
    fn handle_sample(&mut self, t: &Transition<S, A>) {
        let phi_s = self.fa_theta.projector.project(&t.from.state());

        let v = self.fa_theta.evaluate_phi(&phi_s);
        let nv = self.predict_v(t.to.state());

        let td_error = t.reward + self.gamma * nv - v;

        self.update_v(phi_s, td_error)
    }

    fn handle_terminal(&mut self, t: &Transition<S, A>) {
        {
            let phi_s = self.fa_theta.projector.project(&t.from.state());
            let td_error = t.reward - self.fa_theta.evaluate_phi(&phi_s);

            self.update_v(phi_s, td_error);

            self.trace.decay(0.0);
        }

        self.alpha = self.alpha.step();
        self.gamma = self.gamma.step();
    }
}

impl<S, A, P: Projector<S>> Predictor<S, A> for TDLambda<S, P> {
    fn predict_v(&mut self, s: &S) -> f64 { self.fa_theta.evaluate(s).unwrap() }
}

impl<S, P: Projector<S>> Parameterised for TDLambda<S, P> {
    fn weights(&self) -> Matrix<f64> {
        self.fa_theta.weights()
    }
}