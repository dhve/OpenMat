//! SUNDIALS CVODE backend (BDF method), behind the `sundials` feature.
//!
//! Every `unsafe` call in this crate lives in this one module. `CvodeContext`
//! is the only thing that touches raw SUNDIALS handles directly; it owns
//! them and frees them in its `Drop` impl, in reverse creation order, so a
//! solve that returns early on error still cleans up correctly.

use std::os::raw::{c_int, c_void};
use std::ptr;

use sundials_sys::{
    comm_no_mpi, sunindextype, sunrealtype, CVode, CVodeCreate, CVodeFree, CVodeInit,
    CVodeSStolerances, CVodeSetLinearSolver, CVodeSetMaxNumSteps, CVodeSetUserData, N_VDestroy,
    N_VGetArrayPointer, N_VNew_Serial, SUNContext, SUNContext_Create, SUNContext_Free,
    SUNDenseMatrix, SUNLinSolFree, SUNLinSol_Dense, SUNLinearSolver, SUNMatDestroy, SUNMatrix,
    N_Vector, CV_BDF, CV_NORMAL, CV_SUCCESS,
};

use crate::error::SolveError;
use crate::problem::OdeProblem;
use crate::solution::OdeSolution;
use crate::solver::{output_grid, OdeSolver};

/// The right hand side closure, handed to CVODE as an opaque user data
/// pointer and recovered inside `rhs_trampoline`.
struct RhsUserData<'a> {
    rhs: &'a (dyn Fn(f64, &[f64], &mut [f64]) + Send),
    n: usize,
}

/// The C-callable shim CVODE invokes for every derivative evaluation. It
/// only unpacks the SUNDIALS vectors into slices and forwards to the user
/// closure; all the real work happens in safe Rust on the other side.
unsafe extern "C" fn rhs_trampoline(
    t: sunrealtype,
    y: N_Vector,
    ydot: N_Vector,
    user_data: *mut c_void,
) -> c_int {
    let data = &*(user_data as *const RhsUserData);
    let y_slice = std::slice::from_raw_parts(N_VGetArrayPointer(y), data.n);
    let ydot_slice = std::slice::from_raw_parts_mut(N_VGetArrayPointer(ydot), data.n);
    (data.rhs)(t, y_slice, ydot_slice);
    0
}

/// RAII owner of every SUNDIALS handle allocated for one solve. Fields start
/// null and are filled in as setup proceeds, so `Drop` is safe to run at any
/// point, including after a partial, failed setup.
struct CvodeContext {
    sunctx: SUNContext,
    cvode_mem: *mut c_void,
    y: N_Vector,
    matrix: SUNMatrix,
    linsol: SUNLinearSolver,
}

impl CvodeContext {
    fn empty() -> Self {
        Self {
            sunctx: ptr::null_mut(),
            cvode_mem: ptr::null_mut(),
            y: ptr::null_mut(),
            matrix: ptr::null_mut(),
            linsol: ptr::null_mut(),
        }
    }
}

impl Drop for CvodeContext {
    fn drop(&mut self) {
        // Reverse creation order: linear solver and matrix depend on the
        // vector and context; the vector and cvode_mem depend on the
        // context; the context must go last.
        unsafe {
            if !self.cvode_mem.is_null() {
                CVodeFree(&mut self.cvode_mem as *mut *mut c_void);
            }
            if !self.linsol.is_null() {
                SUNLinSolFree(self.linsol);
            }
            if !self.matrix.is_null() {
                SUNMatDestroy(self.matrix);
            }
            if !self.y.is_null() {
                N_VDestroy(self.y);
            }
            if !self.sunctx.is_null() {
                SUNContext_Free(&mut self.sunctx as *mut SUNContext);
            }
        }
    }
}

/// SUNDIALS CVODE using the BDF (backward differentiation formula) linear
/// multistep method, the standard choice for stiff systems.
#[derive(Debug, Clone, Copy)]
pub struct CvodeSolver {
    /// Passed to `CVodeSetMaxNumSteps`: internal step budget between
    /// requested output times before CVODE gives up.
    pub max_num_steps: i64,
}

impl Default for CvodeSolver {
    fn default() -> Self {
        Self {
            max_num_steps: 500_000,
        }
    }
}

/// Maps a non-success CVODE/SUNDIALS return code to a `SolveError`.
fn check(rc: c_int, what: &str) -> Result<(), SolveError> {
    if rc == CV_SUCCESS {
        Ok(())
    } else {
        Err(SolveError::Backend(format!(
            "{what} failed with SUNDIALS code {rc}"
        )))
    }
}

impl OdeSolver for CvodeSolver {
    fn solve(
        &self,
        problem: &OdeProblem,
        n_output_points: usize,
    ) -> Result<OdeSolution, SolveError> {
        let (t0, t1) = problem.t_span;
        if !(t1 > t0) {
            return Err(SolveError::InvalidProblem(
                "t_span end must be greater than t_span start".to_string(),
            ));
        }
        let n = problem.y0.len();
        if n == 0 {
            return Err(SolveError::InvalidProblem(
                "initial state must have at least one component".to_string(),
            ));
        }

        let output_times = output_grid(t0, t1, n_output_points);
        let user_data = RhsUserData {
            rhs: problem.rhs.as_ref(),
            n,
        };

        // Safety: every SUNDIALS call below is used exactly as documented by
        // the sundials-sys crate (mirroring its own test in src/lib.rs) and
        // every handle it allocates is stored in `ctx`, which frees them all
        // in `Drop`, so every early `return` below still cleans up.
        unsafe {
            let mut ctx = CvodeContext::empty();

            let rc = SUNContext_Create(comm_no_mpi(), &mut ctx.sunctx as *mut SUNContext);
            if rc < 0 || ctx.sunctx.is_null() {
                return Err(SolveError::Backend(format!(
                    "SUNContext_Create failed with code {rc}"
                )));
            }

            ctx.y = N_VNew_Serial(n as sunindextype, ctx.sunctx);
            if ctx.y.is_null() {
                return Err(SolveError::Backend("N_VNew_Serial returned null".into()));
            }
            {
                let y_slice = std::slice::from_raw_parts_mut(N_VGetArrayPointer(ctx.y), n);
                y_slice.copy_from_slice(&problem.y0);
            }

            ctx.cvode_mem = CVodeCreate(CV_BDF, ctx.sunctx);
            if ctx.cvode_mem.is_null() {
                return Err(SolveError::Backend("CVodeCreate returned null".into()));
            }

            check(CVodeInit(ctx.cvode_mem, Some(rhs_trampoline), t0, ctx.y), "CVodeInit")?;
            check(
                CVodeSStolerances(ctx.cvode_mem, problem.rtol, problem.atol),
                "CVodeSStolerances",
            )?;

            ctx.matrix = SUNDenseMatrix(n as sunindextype, n as sunindextype, ctx.sunctx);
            if ctx.matrix.is_null() {
                return Err(SolveError::Backend("SUNDenseMatrix returned null".into()));
            }

            ctx.linsol = SUNLinSol_Dense(ctx.y, ctx.matrix, ctx.sunctx);
            if ctx.linsol.is_null() {
                return Err(SolveError::Backend("SUNLinSol_Dense returned null".into()));
            }

            check(
                CVodeSetLinearSolver(ctx.cvode_mem, ctx.linsol, ctx.matrix),
                "CVodeSetLinearSolver",
            )?;
            check(
                CVodeSetUserData(
                    ctx.cvode_mem,
                    &user_data as *const RhsUserData as *mut c_void,
                ),
                "CVodeSetUserData",
            )?;
            check(
                CVodeSetMaxNumSteps(ctx.cvode_mem, self.max_num_steps),
                "CVodeSetMaxNumSteps",
            )?;

            let mut out_t = Vec::with_capacity(output_times.len());
            let mut out_y = Vec::with_capacity(output_times.len());
            let mut tret: sunrealtype = t0;

            for &tau in &output_times {
                if tau <= t0 {
                    // CVode itself refuses to integrate to or past t0 before
                    // any step has been taken; the initial condition is the
                    // answer for every requested point at or before t0.
                    out_t.push(t0);
                    out_y.push(problem.y0.clone());
                    continue;
                }

                // itask = CV_NORMAL: CVODE internally steps at whatever size
                // its own error control picks, past tau if needed, then
                // interpolates (dense output) back to tau. Output points are
                // never used to restrict the integration step size.
                let rc = CVode(ctx.cvode_mem, tau, ctx.y, &mut tret as *mut sunrealtype, CV_NORMAL);
                if rc < 0 {
                    return Err(SolveError::Backend(format!(
                        "CVode failed with code {rc} while integrating to t = {tau}"
                    )));
                }

                let y_slice = std::slice::from_raw_parts(N_VGetArrayPointer(ctx.y), n);
                out_t.push(tret);
                out_y.push(y_slice.to_vec());
            }

            // `ctx` drops here, freeing the linear solver, matrix, vector,
            // CVODE memory, and context in that order.
            Ok(OdeSolution { t: out_t, y: out_y })
        }
    }
}
