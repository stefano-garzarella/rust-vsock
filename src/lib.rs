use nix::errno::Errno;
use nix::{Result, Error};
use nix::sys::socket::{AddressFamily, SockType, SockFlag, SockAddr, MsgFlags};
use nix::sys::socket::{socket, bind, connect, listen, accept, send, recv,
                       getsockname, getpeername};
use std::os::unix::io::RawFd;

pub struct VsockCid {}

impl VsockCid {

    pub fn any() -> u32 {
        libc::VMADDR_CID_ANY
    }

    pub fn hypervisor() -> u32 {
        libc::VMADDR_CID_HYPERVISOR
    }

    pub fn host() -> u32 {
        libc::VMADDR_CID_HOST
    }
}

pub struct Vsock {
    fd: RawFd
}

impl Vsock {
    pub fn new() -> Self {
        let socket_fd = socket(AddressFamily::Vsock, SockType::Stream,
                               SockFlag::empty(), None).unwrap();
        Vsock { fd: socket_fd, }
    }

    pub fn raw_fd(&self) -> RawFd {
        self.fd
    }

    pub fn connect(&self, cid: u32, port: u32) -> Result<()> {
        let sockaddr = SockAddr::new_vsock(cid, port);

        return connect(self.fd, &sockaddr);
    }

    pub fn accept(&self) -> Result<Vsock> {
        let client_fd = accept(self.fd)?;

        Ok(Vsock {fd: client_fd})
    }

    pub fn bind(&self, cid: u32, port: u32) -> Result<()> {
        let sockaddr = SockAddr::new_vsock(cid, port);

        return bind(self.fd, &sockaddr);
    }

    pub fn getsockname(&self) -> Result<(u32, u32)> {
        let sockaddr = getsockname(self.fd)?;

        if let SockAddr::Vsock(addr) = sockaddr {
            return Ok((addr.cid(), addr.port()));
        } else {
            return Err(Error::Sys(Errno::EINVAL));
        }
    }

    pub fn getpeername(&self) -> Result<(u32, u32)> {
        let sockaddr = getpeername(self.fd)?;

        if let SockAddr::Vsock(addr) = sockaddr {
            return Ok((addr.cid(), addr.port()));
        } else {
            return Err(Error::Sys(Errno::EINVAL));
        }
    }

    pub fn listen(&self, backlog: usize) -> Result<()> {
        listen(self.fd, backlog)
    }

    pub fn recv(&self, buf: &mut [u8], flags: MsgFlags) -> Result<usize> {
        recv(self.fd, buf, flags)
    }

    pub fn send(&self, buf: &[u8], flags: MsgFlags) -> Result<usize> {
        send(self.fd, buf, flags)
    }
}

impl Drop for Vsock {

    fn drop(&mut self) {
        let _ = nix::unistd::close(self.fd);
    }

}
