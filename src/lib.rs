use futures::{
  channel::mpsc::{self, Sender},
  future::{Either, FutureExt},
  stream::{self, StreamExt},
  Future, Stream,
};

pub fn gen_z_sender<
  'fut,
  T: 'fut + Send,
  Fut: Future<Output = ()> + 'fut + Send,
  Gen: FnOnce(Sender<T>) -> Fut,
>(
  gen: Gen,
) -> impl Stream<Item = T> + 'fut {
  let (send, recv) = mpsc::channel::<T>(0); // 0 + #senders
  let fut: Fut = gen(send);
  // HACK: Join as a stream to ensure future and outputs are polled evenly, and filter out fut's result
  let fut = fut.map(Either::Left).into_stream();
  let joined = stream::select(fut, recv.map(Either::Right)).filter_map(|x| {
    futures::future::ready(match x {
      Either::Left(_) => None,
      Either::Right(y) => Some(y),
    })
  });
  joined.fuse()
}

/// Just provides a nicer panic message for unexpected failures, and makes lifetimes more obvious
async fn send_infallible<T: Send>(sender: &mut Sender<T>, item: T) -> () {
  use futures::sink::SinkExt;
  SinkExt::send(sender, item)
    .await
    .expect("Infallible sends should only be used where they cannot fail")
}

/// A wrapper around a Sender to restrict its behaviour to infallible sends; a "yield" helper
pub struct Yielder<T: Send> {
  sender: Sender<T>,
}

impl<T: Send> Yielder<T> {
  pub fn new(sender: Sender<T>) -> Yielder<T> {
    Yielder { sender }
  }

  pub fn send(&mut self, item: T) -> impl Future<Output = ()> + '_ {
    send_infallible(&mut self.sender, item)
  }

  // TODO: Consider implementing a feature that implements trait-Fn to allow direct calls
}

/// Given an infallible future and a yield helper, produce a stream until the future arrives
pub fn gen_z<
  'fut,
  T: 'fut + Send,
  Fut: Future<Output = ()> + 'fut + Send,
  Gen: 'fut + FnOnce(Yielder<T>) -> Fut,
>(
  gen: Gen,
) -> impl Stream<Item = T> + 'fut {
  gen_z_sender(|sender| gen(Yielder::<T>::new(sender)))
}
