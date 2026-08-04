use super::write_all_cooperative;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use tokio::{
    io::AsyncWrite,
    task::coop::{consume_budget, has_budget_remaining},
};

#[derive(Default)]
struct CompleteWriter {
    output: Vec<u8>,
}

impl AsyncWrite for CompleteWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.output.extend_from_slice(input);
        Poll::Ready(Ok(input.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn exhausted_budget_suspends_before_the_next_external_write() {
    while has_budget_remaining() {
        consume_budget().await;
    }

    let mut writer = CompleteWriter::default();
    let mut future = Box::pin(write_all_cooperative(&mut writer, b"committed-once"));
    let mut context = Context::from_waker(Waker::noop());

    assert!(Future::poll(future.as_mut(), &mut context).is_pending());
    drop(future);
    assert!(writer.output.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn final_successful_write_has_no_following_internal_suspension() {
    let mut writer = CompleteWriter::default();
    let mut future = Box::pin(write_all_cooperative(&mut writer, b"committed-once"));
    let mut context = Context::from_waker(Waker::noop());

    assert!(matches!(
        Future::poll(future.as_mut(), &mut context),
        Poll::Ready(Ok(()))
    ));
    drop(future);
    assert_eq!(writer.output, b"committed-once");
}
