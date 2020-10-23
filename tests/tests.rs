#[test]
fn test_gen_z_sender() {
  use futures::{executor::block_on, SinkExt, StreamExt};
  use gen_z::gen_z_sender;

  let zoomer: Vec<i32> = block_on(
    gen_z_sender(|mut z| async move {
      z.send(1).await.unwrap();
      z.send(2).await.unwrap();
      z.send(3).await.unwrap();
    })
    .collect(),
  );
  assert_eq!(zoomer, vec![1, 2, 3]);
}

#[test]
fn test_gen_z_yielder() {
  use futures::{executor::block_on, StreamExt};
  use gen_z::gen_z;

  let zoomer: Vec<i32> = block_on(
    gen_z(|mut z| async move {
      z.send(1).await;
      z.send(2).await;
      z.send(3).await;
    })
    .collect(),
  );
  assert_eq!(zoomer, vec![1, 2, 3]);
}

#[test]
// Prove that streams execute asynchronously and are compatible with synchronization objects
fn test_simultaneous_streaming() {
  use futures::{executor::block_on, stream, StreamExt};
  use gen_z::gen_z;

  let zoomer: Vec<i32> = block_on({
    let (sa, rb) = futures::channel::oneshot::channel::<()>();
    let (sb, ra) = futures::channel::oneshot::channel::<()>();
    // `a` sends 1, triggers b, waits until `b` sends 2 and triggers `a`, which then sends 3
    let a = gen_z(|mut z| async move {
      z.send(1).await;
      sa.send(()).unwrap();
      ra.await.unwrap();
      z.send(3).await;
    });
    let b = gen_z(|mut z| async move {
      // Wait for the signal from `a`, then send
      rb.await.unwrap();
      z.send(2).await;
      sb.send(()).unwrap();
    });
    // Combine the two streams, polling until both end, and collect them into a vector
    stream::select(a, b).collect::<Vec<i32>>()
  });
  assert_eq!(zoomer, vec![1, 2, 3]);
}
