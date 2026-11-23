//! Checked Ogawa block access, independent of Alembic scene schemas.

use super::{Error, invalid};

pub const DATA: u64 = 1 << 63;
const HEADER: &[u8; 8] = b"Ogawa\xff\0\x01";

#[derive(Clone, Copy)]
pub struct Group<'a>(&'a [u8]);

impl Group<'_> {
    pub fn len(self) -> usize {
        self.0.len() / 8
    }

    pub fn at(self, i: usize) -> Result<u64, Error> {
        let at = i
            .checked_mul(8)
            .ok_or_else(|| invalid("group index overflow"))?;
        let mut c = Cursor(
            self.0
                .get(at..)
                .ok_or_else(|| invalid("missing group child"))?,
        );
        c.u64()
    }
}

pub struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn open(bytes: &'a [u8]) -> Result<(Self, Group<'a>), Error> {
        if !bytes.starts_with(HEADER) {
            return Err(invalid("expected a frozen Ogawa version 1 archive"));
        }
        let mut header = Cursor(&bytes[8..]);
        let reader = Self { bytes };
        let root = reader.group(header.u64()?)?;
        Ok((reader, root))
    }

    fn payload(&self, offset: u64, stride: usize) -> Result<&'a [u8], Error> {
        if offset == 0 {
            return Ok(&[]);
        }
        let start = usize::try_from(offset).map_err(|_| invalid("Ogawa offset overflow"))?;
        if start < 16 {
            return Err(invalid("Ogawa block points into the header"));
        }
        let mut c = Cursor(
            self.bytes
                .get(start..)
                .ok_or_else(|| invalid("Ogawa pointer outside archive"))?,
        );
        let len = usize::try_from(c.u64()?)
            .ok()
            .and_then(|n| n.checked_mul(stride))
            .ok_or_else(|| invalid("Ogawa block length overflow"))?;
        c.take(len)
    }

    pub fn group(&self, ptr: u64) -> Result<Group<'a>, Error> {
        if ptr & DATA != 0 {
            return Err(invalid("expected an Ogawa group"));
        }
        Ok(Group(self.payload(ptr, 8)?))
    }

    pub fn data(&self, ptr: u64) -> Result<&'a [u8], Error> {
        if ptr & DATA == 0 {
            return Err(invalid("expected an Ogawa data block"));
        }
        self.payload(ptr & !DATA, 1)
    }
}

pub struct Cursor<'a>(pub &'a [u8]);

impl<'a> Cursor<'a> {
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let (head, tail) = self
            .0
            .split_at_checked(n)
            .ok_or_else(|| invalid("truncated Ogawa data"))?;
        self.0 = tail;
        Ok(head)
    }

    pub fn string(&mut self, n: usize) -> Result<&'a str, Error> {
        std::str::from_utf8(self.take(n)?).map_err(|_| invalid("invalid Alembic UTF-8"))
    }

    pub fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    pub fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }

    pub fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("eight bytes"),
        ))
    }

    pub fn f64(&mut self) -> Result<f64, Error> {
        Ok(f64::from_bits(self.u64()?))
    }

    pub fn width(&mut self, width: u32) -> Result<usize, Error> {
        match width {
            0 => Ok(self.u8()?.into()),
            1 => Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("two bytes")).into()),
            2 => Ok(self.u32()? as usize),
            _ => Err(invalid("invalid Alembic header integer width")),
        }
    }
}

pub struct Writer(Vec<u8>);

impl Writer {
    pub fn new() -> Self {
        let mut buf = HEADER.to_vec();
        buf.extend([0; 8]);
        Self(buf)
    }

    pub fn data(&mut self, bytes: &[u8]) -> u64 {
        if bytes.is_empty() {
            return DATA;
        }
        let start = self.0.len() as u64;
        self.0.extend((bytes.len() as u64).to_le_bytes());
        self.0.extend(bytes);
        start | DATA
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn group(&mut self, ptrs: &[u64]) -> u64 {
        if ptrs.is_empty() {
            return 0;
        }
        let start = self.0.len() as u64;
        self.0.extend((ptrs.len() as u64).to_le_bytes());
        self.0.extend(ptrs.iter().flat_map(|p| p.to_le_bytes()));
        start
    }

    pub fn finish(mut self, root: u64) -> Vec<u8> {
        self.0[8..16].copy_from_slice(&root.to_le_bytes());
        self.0
    }
}
