use std::alloc::{GlobalAlloc, Layout, System};

#[derive(Debug)]
/// Single Memory Block of Heterogeneous Data
pub struct HetSingleMemBlock {
    base: *mut u8,
    layout: Layout,
}

impl HetSingleMemBlock {
    /// Allocates a new block for a group of heterogeneous data
    pub fn alloc(layouts: &[Layout]) -> Self {
        assert!(!layouts.is_empty(), "Layouts can't be empty");

        let mut block_layout = Layout::from_size_align(0, 1).expect("Invalid Layout");

        for layout in layouts {
            let (new_layout, _ /* offset */) = block_layout
                .extend(*layout)
                .expect("Invalid Layout Construction!");
            block_layout = new_layout;
        }

        let block_layout = block_layout.pad_to_align();

        let base = unsafe { System.alloc(block_layout) };
        assert!(!base.is_null(), "Failed to allocate!");

        Self {
            base,
            layout: block_layout,
        }
    }

    #[inline(always)]
    pub fn get_base(&self) -> *mut u8 {
        self.base
    }

    #[inline(always)]
    pub fn get_layout(&self) -> &Layout {
        &self.layout
    }

    /// Get an iterator on the block memory based on data layouts
    ///
    /// # Safety
    /// The layouts used here must be the same as the ones used for the blocks allocation
    pub unsafe fn get_layout_iter<'a>(&'a self, layouts: &'a [Layout]) -> BlockLayoutIterator<'a> {
        BlockLayoutIterator {
            block: self,
            layouts,
            current_layout: Layout::from_size_align(0, 1).expect("Invalid Layout"),
            index: 0,
        }
    }
}

impl Drop for HetSingleMemBlock {
    fn drop(&mut self) {
        if !self.base.is_null() {
            unsafe {
                System.dealloc(self.base, self.layout);
            }
            self.base = std::ptr::null_mut();
        }
    }
}

#[derive(Debug)]
pub struct BlockLayoutIterator<'a> {
    block: &'a HetSingleMemBlock,
    layouts: &'a [Layout],
    current_layout: Layout,
    index: usize,
}

impl<'a> Iterator for BlockLayoutIterator<'a> {
    type Item = *mut u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.layouts.len() {
            return None;
        }

        let (next_layout, next_offset) = self
            .current_layout
            .extend(self.layouts[self.index])
            .expect("Invalid Layout!");

        self.current_layout = next_layout;
        self.index += 1;

        Some(unsafe { self.block.get_base().byte_add(next_offset) })
    }
}

#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;
    use std::alloc::Layout;
    use std::mem::MaybeUninit;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::Ordering;

    use super::HetSingleMemBlock;

    #[derive(Debug, PartialEq)]
    struct A {
        pub a: u64,
        pub b: u32,
        pub c: bool,
    }

    lazy_static! {
        static ref A_COUNTER: AtomicI32 = AtomicI32::new(0);
    }

    impl Default for A {
        fn default() -> Self {
            A_COUNTER.fetch_add(1, Ordering::AcqRel);
            Self {
                a: 123456789,
                b: 987654321,
                c: false,
            }
        }
    }

    impl A {
        pub fn check(&self) {
            assert_eq!(self.a, 123456789);
            assert_eq!(self.b, 987654321);
            assert!(!self.c);
        }
    }
    impl Drop for A {
        fn drop(&mut self) {
            let res = A_COUNTER.fetch_add(-1, Ordering::AcqRel);
            assert!(res >= 0, "Struct A dropped too many times!");
        }
    }

    #[derive(Debug, PartialEq)]
    #[repr(C)]
    struct B {
        pub a: bool,
        pub b: u32,
        pub c: u64,
    }

    lazy_static! {
        static ref B_COUNTER: AtomicI32 = AtomicI32::new(0);
    }

    impl Default for B {
        fn default() -> Self {
            B_COUNTER.fetch_add(1, Ordering::AcqRel);
            Self {
                a: true,
                b: 123456789,
                c: 987654321,
            }
        }
    }

    impl B {
        pub fn check(&self) {
            assert!(self.a);
            assert_eq!(self.b, 123456789);
            assert_eq!(self.c, 987654321);
        }
    }

    impl Drop for B {
        fn drop(&mut self) {
            let res = B_COUNTER.fetch_add(-1, Ordering::AcqRel);
            assert!(res >= 0, "Struct B dropped too many times!");
        }
    }

    #[test]
    fn test_het_mem_block() {
        let layouts = [
            Layout::new::<MaybeUninit<A>>(),
            Layout::new::<MaybeUninit<B>>(),
        ];

        let block = HetSingleMemBlock::alloc(&layouts);

        let offsets = unsafe { block.get_layout_iter(&layouts).collect::<Vec<*mut u8>>() };

        let a_uninit = unsafe { &mut *offsets[0].cast::<MaybeUninit<A>>() };
        let b_uninit = unsafe { &mut *offsets[1].cast::<MaybeUninit<B>>() };

        let a = a_uninit.write(A::default());
        let b = b_uninit.write(B::default());

        assert_eq!(A_COUNTER.load(Ordering::Acquire), 1);
        assert_eq!(B_COUNTER.load(Ordering::Acquire), 1);

        a.check();
        b.check();

        let a_uninit2 = unsafe { &mut *offsets[0].cast::<MaybeUninit<A>>() };
        let b_uninit2 = unsafe { &mut *offsets[1].cast::<MaybeUninit<B>>() };

        let a2 = unsafe { a_uninit2.assume_init_ref() };
        let b2 = unsafe { b_uninit2.assume_init_ref() };

        assert_eq!(a, a2);
        assert_eq!(b, b2);

        unsafe { a_uninit.assume_init_drop() };
        unsafe { b_uninit.assume_init_drop() };

        assert_eq!(A_COUNTER.load(Ordering::Acquire), 0);
        assert_eq!(B_COUNTER.load(Ordering::Acquire), 0);
    }
}
