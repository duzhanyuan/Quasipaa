use super::Rx;
use futures::prelude::*;
use std::net::SocketAddr;
use std::{pin::Pin, io::Error};
use std::task::{Context, Poll};
use tokio::{net::TcpStream, io::AsyncWrite};
use transport::Transport;

/// Data advancement
/// 
/// Push the event and data of the instance 
/// to other business backends through TCPSocket.
/// 
/// TODO: 单路TCP负载能力有限，
/// 计划使用多路合并来提高传输能力;
pub struct Forward {
    transport: Transport,
    stream: TcpStream,
    receiver: Rx,
}

impl Forward {
    /// Create an example of data advancement
    /// 
    /// Specify a remote address and data pipeline bus 
    /// to create an instance, which is responsible for 
    /// serializing the data into tcp data stream and 
    /// pushing it to other business backends.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use forward::Forward;
    /// use tokio::sync::mpsc;
    ///
    /// let addr = "127.0.0.1:1936".parse().unwrap();
    /// let (_, receiver) = mpsc::unbounded_channel();
    /// let forward = Forward::new(addr, receiver).await?;
    /// tokio::spawn(forward);
    /// ```
    pub async fn new(addr: SocketAddr, receiver: Rx) -> Result<Self, Error> {
        Ok(Self {
            receiver,
            transport: Transport::new(),
            stream: TcpStream::connect(addr).await?
        })
    }

    /// Send data to TcpSocket
    ///
    /// Write Tcp data to TcpSocket.
    /// Check whether the writing is completed, 
    // if not completely written, write the rest.
    ///
    /// TODO: 异常处理未完善, 未处理意外情况，可能会出现死循环;
    #[rustfmt::skip]
    fn send<'b>(&mut self, ctx: &mut Context<'b>, data: &[u8]) {
        let mut offset: usize = 0;
        let length = data.len();
        loop {
            match Pin::new(&mut self.stream).poll_write(ctx, &data) {
                Poll::Ready(Ok(s)) => match &offset + &s >= length {
                    false => { offset += s; },
                    true => { break; }
                }, _ => (),
            }
        }
    }

    /// Handling pipeline messages
    /// 
    /// Try to process the backlog message in the 
    /// pipeline, and serialize it into tcp protocol 
    /// packet through the data transfer module to 
    /// send to tcpsocket.
    #[rustfmt::skip]
    fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Poll::Ready(Some((flag, data))) = Pin::new(&mut self.receiver).poll_next(ctx) {
            let buffer = self.transport.encoder(data, flag);
            self.send(ctx, &buffer);
        }
    }
}

impl Future for Forward {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
