use futures::compat::Future01CompatExt;
use std::convert::TryInto;
use tokio_io::io::read_exact;
use tokio_io::io::write_all;

use super::super::codec::write_address;
use crate::consts;
use crate::consts::Reply;
use crate::socks::read_socks_address;
use crate::socks::Address;
use crate::socks::SocksError;
use crate::socks::TcpRequestHeader;
use crate::socks::TcpResponseHeader;
use bytes::BufMut;
use bytes::BytesMut;
use std::net::Shutdown;
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub async fn read_command_async(
    mut s: TcpStream,
    peer: SocketAddr,
) -> Result<(TcpStream, TcpRequestHeader), SocksError> {
    let (_s, buf) = await!(read_exact(&mut s, [0u8; 3]).compat())?;
    let ver = buf[0];
    if ver != consts::SOCKS5_VERSION {
        await!(write_command_response_async(
            &mut s,
            Reply::ConnectionRefused,
            peer
        ))?;
        s.shutdown(Shutdown::Both)?;
        return Err(SocksError::SocksVersionNoSupport { ver });
    }
    let cmd = buf[1].try_into();
    if let Err(e) = cmd {
        let e: SocksError = e;
        await!(write_command_response_async(
            &mut s,
            Reply::CommandNotSupported,
            peer
        ))?;
        s.shutdown(Shutdown::Both)?;
        return Err(e.into());
    }
    let cmd = cmd.unwrap();
    await!(write_command_response_async(&mut s, Reply::SUCCEEDED, peer))?;
    let address = await!(read_socks_address(&mut s))?;
    let header = TcpRequestHeader {
        command: cmd,
        address,
    };
    Ok((s, header))
}

async fn write_command_response_async(
    s: &mut TcpStream,
    rep: Reply,
    addr: SocketAddr,
) -> Result<(), SocksError> {
    let addr = Address::SocketAddress(addr);
    let resp = TcpResponseHeader::new(rep, addr);
    let mut buf = BytesMut::with_capacity(resp.len());
    buf.put_slice(&[consts::SOCKS5_VERSION, resp.reply as u8, 0x00]);
    write_address(&resp.address, &mut buf);
    await!(write_all(s, buf).compat())?;
    Ok(())
}
