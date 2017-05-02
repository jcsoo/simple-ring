#![allow(dead_code)]
#![feature(const_fn)]
#![no_std]

use core::cell::Cell;
use core::cmp;

macro_rules! static_ring_buf {
    ($size:expr, $ty:ty, $zero:expr) => {
        {
            static mut BUF: [$ty; $size] = [$zero; $size];
            static mut RBUF: Option<RingBuf<$ty>> = None;
            unsafe {
                RBUF = Some(
                    RingBuf {
                        reader: Cell::new(0),
                        writer: Cell::new(0),
                        buffer: &mut BUF,
                    }
                );
                RBUF.as_ref().unwrap().pair()
            }
        }
    }
}

pub struct RingBuf<T: Copy> {
    reader: Cell<usize>,
    writer: Cell<usize>,
    buffer: *mut [T],
}

impl<T: Copy> RingBuf<T> {
    pub fn pair(&self) -> (RingBufReader<T>, RingBufWriter<T>) {
        (
            RingBufReader { rb: self},
            RingBufWriter { rb: self}
        )
    }

    fn as_ref(&self) -> &[T] {
        unsafe { &*self.buffer }
    }

    fn as_mut(&self) -> &mut [T]{
        unsafe { &mut *self.buffer }
    }    

    fn cap(&self) -> usize {
        self.as_ref().len()
    }

    fn len(&self) -> usize {
        self.writer.get().wrapping_sub(self.reader.get())
    }

    fn rem(&self) -> usize {
        self.cap().wrapping_sub(self.len())
    }

    fn is_empty(&self) -> bool {
        self.reader.get() == self.writer.get()
    }

    fn is_full(&self) -> bool {
        self.len() == self.cap()
    }

    fn phy(&self, index: usize) -> usize {
        index % self.cap()
    }

    fn incr_reader(&self) {
        assert!(!self.is_empty(), "Attempted to increment empty reader");
        self.reader.set(self.reader.get().wrapping_add(1));
    }

    fn incr_writer(&self) {        
        assert!(!self.is_full(), "Attempted to increment full writer");
        self.writer.set(self.writer.get().wrapping_add(1));     
    }

    pub fn enqueue(&self, value: T) -> bool {
        if self.is_full() {
            false
        } else {
            let writer = self.phy(self.writer.get());
            self.as_mut()[writer] = value;
            self.incr_writer();
            true
        }
    }

    pub fn dequeue(&self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let reader = self.phy(self.reader.get());
            let value = self.as_ref()[reader];
            self.incr_reader();
            Some(value)
        }
    }

    pub fn write(&self, buf: &[T]) -> usize {
        let n = cmp::min(self.rem(), buf.len());
        for i in 0..n {
            self.enqueue(buf[i]);
        }
        n
    }

    pub fn read(&self, buf: &mut [T]) -> usize {
        let n = cmp::min(self.len(), buf.len());
        for i in 0..n {
            buf[i] = self.dequeue().expect("Ring buffer is empty");
        }
        n
    }    
}

pub struct RingBufReader<'a, T: 'a + Copy> {
    rb: &'a RingBuf<T>,
}

impl<'a, T: Copy> RingBufReader<'a, T> {
    pub fn dequeue(&self) -> Option<T> {
        self.rb.dequeue()
    }
    pub fn read(&self, buf: &mut [T]) -> usize {
        self.rb.read(buf)
    }
}

pub struct RingBufWriter<'a, T: 'a + Copy> {
    rb: &'a RingBuf<T>,
}

impl<'a, T: Copy> RingBufWriter<'a, T> {
    pub fn enqueue(&self, value: T) -> bool {
        self.rb.enqueue(value)
    }
    pub fn write(&self, buf: &[T]) -> usize {    
        self.rb.write(buf)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static() {
        let (r, w) = static_ring_buf!(16, u8, 0);
        w.write(b"Hello, World");
        let mut dst = [0u8; 64];
        let n = r.read(&mut dst);
        assert_eq!(&dst[..n], b"Hello, World");
    }


    #[test]
    fn test_enqueue_dequeue() {
        let (r, w) = static_ring_buf!(16, u8, 0);
        
        for i in 0..16 {
            assert_eq!(w.enqueue(i as u8), true);
        }
        assert_eq!(w.enqueue(0), false);
        for i in 0..16 {
            assert_eq!(r.dequeue(), Some(i as u8));
        }
        assert_eq!(r.dequeue(), None);
    }

     #[test]
    fn test_enqueue_dequeue_u32() {
        let (r, w) = static_ring_buf!(16, u32, 0);
        
        for i in 0..16 {
            assert_eq!(w.enqueue(i as u32), true);
        }
        assert_eq!(w.enqueue(0), false);
        for i in 0..16 {
            assert_eq!(r.dequeue(), Some(i as u32));
        }
        assert_eq!(r.dequeue(), None);
    }   

    #[test]
    fn test_write_read() {
        let (r, w) = static_ring_buf!(16, u8, 0);

        let src: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let mut dst = [0u8; 16];

        w.write(&src);
        let n = r.read(&mut dst);
        assert_eq!(n, 16);
        for i in 0..16 {
            assert_eq!(src[i], dst[i]);
        }
    }

    pub struct Driver<'a> {
        w: RingBufWriter<'a, u8>
    }

}