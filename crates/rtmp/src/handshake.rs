use crate::{Error, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const SIZE: usize = 1536;

fn block() -> [u8; SIZE] {
    // No encryption/authentication is implied by the RTMP simple handshake.
    // Protocol timestamps are zero; the deterministic padding is only echoed.
    let mut value = [0; SIZE];
    for (i, byte) in value.iter_mut().enumerate().skip(8) {
        *byte = (i as u8).wrapping_mul(73).wrapping_add(19);
    }
    value
}

/// RTMP version 3 simple handshake, used by FFmpeg/OBS unencrypted RTMP.
/// The caller must impose its own handshake deadline.
pub async fn server_handshake<S: AsyncRead + AsyncWrite + Unpin>(stream: &mut S) -> Result<()> {
    if stream.read_u8().await? != 3 {
        return Err(Error::Protocol("unsupported handshake version"));
    }
    let mut c1 = [0; SIZE];
    stream.read_exact(&mut c1).await?;
    let s1 = block();
    stream.write_all(&[3]).await?;
    stream.write_all(&s1).await?;
    stream.write_all(&c1).await?;
    stream.flush().await?;
    let mut c2 = [0; SIZE];
    stream.read_exact(&mut c2).await?;
    // Some clients use the complex C1 format and accept a simple response.
    // C2 time2 differs by design; only the random echo is compared.
    if c2[8..] != s1[8..] {
        return Err(Error::Protocol("handshake echo mismatch"));
    }
    Ok(())
}

pub async fn client_handshake<S: AsyncRead + AsyncWrite + Unpin>(stream: &mut S) -> Result<()> {
    let c1 = block();
    stream.write_all(&[3]).await?;
    stream.write_all(&c1).await?;
    stream.flush().await?;
    if stream.read_u8().await? != 3 {
        return Err(Error::Protocol("unsupported handshake version"));
    }
    let mut s1 = [0; SIZE];
    let mut s2 = [0; SIZE];
    stream.read_exact(&mut s1).await?;
    stream.read_exact(&mut s2).await?;
    if s2[8..] != c1[8..] {
        return Err(Error::Protocol("handshake echo mismatch"));
    }
    stream.write_all(&s1).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn simple_handshake_survives_small_transport_buffers() {
        let (mut client, mut server) = tokio::io::duplex(17);
        let task = tokio::spawn(async move { server_handshake(&mut server).await });
        client_handshake(&mut client).await.unwrap();
        task.await.unwrap().unwrap();
    }
}
