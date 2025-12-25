use gpui::{Context, Task};
use std::{marker::PhantomData, time::Duration};

/// UI-thread debouncer modeled after Zed's `DebouncedDelay`: coalesces work
/// and only applies the latest invocation after `delay`.
pub struct Debouncer<E: 'static> {
    delay: Duration,
    task: Option<Task<()>>,
    _phantom: PhantomData<E>,
}

impl<E: 'static> Debouncer<E> {
    pub fn new(delay: Duration) -> Self {
        Self {
            delay,
            task: None,
            _phantom: PhantomData,
        }
    }

    pub fn schedule<F>(&mut self, cx: &mut Context<E>, mut job: F)
    where
        F: 'static + Send + FnMut(&mut E, &mut Context<E>),
    {
        let previous = self.task.take();
        let delay = self.delay;

        self.task = Some(cx.spawn(async move |entity, cx| {
            if let Some(previous) = previous {
                previous.await;
            }

            cx.background_executor().timer(delay).await;

            let _ = entity.update(cx, |view, cx| job(view, cx));
        }));
    }
}
