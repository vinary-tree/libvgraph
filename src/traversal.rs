use crate::control::{Unbounded, WorkControl};
use crate::{ComputeError, CsrGraph, DenseId, ExecutionControl, GraphError};

impl<K: Ord> CsrGraph<K> {
    /// Returns deterministic breadth-first order from `start` without a work cap.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Invalid`] when `start` is outside the graph.
    pub fn breadth_first(&self, start: DenseId) -> Result<Vec<DenseId>, ComputeError> {
        self.breadth_first_impl(start, &mut Unbounded)
    }

    /// Returns deterministic breadth-first order under work and cancellation controls.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Invalid`] for an out-of-range `start`, or
    /// [`ComputeError::Incomplete`] when the supplied control stops traversal.
    pub fn breadth_first_with_control(
        &self,
        start: DenseId,
        control: ExecutionControl<'_>,
    ) -> Result<Vec<DenseId>, ComputeError> {
        self.breadth_first_impl(start, &mut control.meter())
    }

    /// Returns deterministic iterative depth-first preorder without a work cap.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Invalid`] when `start` is outside the graph.
    pub fn depth_first_preorder(&self, start: DenseId) -> Result<Vec<DenseId>, ComputeError> {
        self.depth_first_preorder_impl(start, &mut Unbounded)
    }

    /// Returns deterministic iterative depth-first preorder under controls.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Invalid`] for an out-of-range `start`, or
    /// [`ComputeError::Incomplete`] when the supplied control stops traversal.
    pub fn depth_first_preorder_with_control(
        &self,
        start: DenseId,
        control: ExecutionControl<'_>,
    ) -> Result<Vec<DenseId>, ComputeError> {
        self.depth_first_preorder_impl(start, &mut control.meter())
    }

    /// Returns a deterministic iterative depth-first forest over every vertex.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Invalid`] only if an internal graph invariant is
    /// violated. A value constructed by libvgraph satisfies those invariants.
    pub fn depth_first_forest(&self) -> Result<Vec<DenseId>, ComputeError> {
        self.depth_first_forest_impl(&mut Unbounded)
    }

    /// Returns the same depth-first forest under work and cancellation controls.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Incomplete`] when the supplied control stops the
    /// traversal, or [`ComputeError::Invalid`] for an internal invariant failure.
    pub fn depth_first_forest_with_control(
        &self,
        control: ExecutionControl<'_>,
    ) -> Result<Vec<DenseId>, ComputeError> {
        self.depth_first_forest_impl(&mut control.meter())
    }

    fn breadth_first_impl<C: WorkControl>(
        &self,
        start: DenseId,
        control: &mut C,
    ) -> Result<Vec<DenseId>, ComputeError> {
        ensure_start(self, start)?;
        let mut visited = vec![false; self.vertex_count()];
        let mut order = Vec::with_capacity(self.vertex_count());
        visited[start.index()] = true;
        order.push(start);
        let mut queue_index = 0usize;
        while queue_index < order.len() {
            control.step()?;
            let vertex = order[queue_index];
            queue_index += 1;
            for &successor in self.successors_unchecked(vertex) {
                control.step()?;
                if !visited[successor.index()] {
                    visited[successor.index()] = true;
                    order.push(successor);
                }
            }
        }
        Ok(order)
    }

    fn depth_first_preorder_impl<C: WorkControl>(
        &self,
        start: DenseId,
        control: &mut C,
    ) -> Result<Vec<DenseId>, ComputeError> {
        ensure_start(self, start)?;
        let mut visited = vec![false; self.vertex_count()];
        let mut stack = Vec::with_capacity(self.vertex_count());
        let mut order = Vec::with_capacity(self.vertex_count());
        visited[start.index()] = true;
        stack.push(start);
        while let Some(vertex) = stack.pop() {
            control.step()?;
            order.push(vertex);
            for &successor in self.successors_unchecked(vertex).iter().rev() {
                control.step()?;
                if !visited[successor.index()] {
                    visited[successor.index()] = true;
                    stack.push(successor);
                }
            }
        }
        Ok(order)
    }

    fn depth_first_forest_impl<C: WorkControl>(
        &self,
        control: &mut C,
    ) -> Result<Vec<DenseId>, ComputeError> {
        let mut visited = vec![false; self.vertex_count()];
        let mut order = Vec::with_capacity(self.vertex_count());
        let mut stack = Vec::with_capacity(self.vertex_count());
        for root in 0..self.vertex_count_u32() {
            let root = DenseId::from_raw(root);
            if visited[root.index()] {
                continue;
            }
            visited[root.index()] = true;
            stack.push(root);
            while let Some(vertex) = stack.pop() {
                control.step()?;
                order.push(vertex);
                for &successor in self.successors_unchecked(vertex).iter().rev() {
                    control.step()?;
                    if !visited[successor.index()] {
                        visited[successor.index()] = true;
                        stack.push(successor);
                    }
                }
            }
        }
        Ok(order)
    }
}

fn ensure_start<K: Ord>(graph: &CsrGraph<K>, start: DenseId) -> Result<(), GraphError> {
    if start.get() >= graph.vertex_count_u32() {
        return Err(GraphError::DenseIdOutOfRange {
            id: start,
            vertex_count: graph.vertex_count_u32(),
        });
    }
    Ok(())
}
