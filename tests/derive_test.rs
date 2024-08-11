use core::cell::{Ref, RefCell};
use core::ptr::drop_in_place;

#[derive(incrstruct_derive::IncrStruct)]
pub struct AStruct<'a> {
    #[borrows(b)]
    pub c: &'a i32,

    #[borrows(head1, head2)]
    pub b: Ref<'a, i32>,

    pub head1: RefCell<i32>,
    pub head2: i64,

    #[header]
    hdr: incrstruct::Header,
}

impl<'a> AStructInit<'a> for AStruct<'a> {
    fn init_field_c(b: &'a Ref<'a, i32>) -> &'a i32 {
        b
    }

    fn init_field_b(_head2: &'a i64, head1: &'a RefCell<i32>) -> Ref<'a, i32> {
        head1.borrow()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uninit_works() {
        let mut a = unsafe { AStruct::new_uninit(4711, RefCell::new(42)) };
        let aref = incrstruct::ensure_init(&mut a);

        assert_eq!(*aref.head1.borrow(), 42);
        assert_eq!(*aref.b, 42);
        assert_eq!(*aref.c, 42);

        unsafe { drop_in_place(aref) };
    }

    #[test]
    fn drop_uninit_in_place_works() {
        let a = unsafe { AStruct::new_uninit(4711, RefCell::new(42)) };

        assert_eq!(*(unsafe { &*a.as_ptr() }).head1.borrow(), 42);

        AStruct::drop_uninit_in_place(a);
    }

    #[test]
    fn new_box_works() {
        let a = AStruct::new_box(4711, RefCell::new(42));

        assert_eq!(*a.head1.borrow(), 42);
        assert_eq!(*a.b, 42);
        assert_eq!(*a.c, 42);
    }

    #[test]
    fn new_rc_works() {
        let a = AStruct::new_rc(4711, RefCell::new(42));

        assert_eq!(*a.head1.borrow(), 42);
        assert_eq!(*a.b, 42);
        assert_eq!(*a.c, 42);
    }

    #[test]
    fn force_init_works() {
        let a = AStruct::new_box(4711, RefCell::new(42));
        let mut b = *a;

        incrstruct::force_init(&mut b);

        assert_eq!(*b.head1.borrow(), 42);
        assert_eq!(*b.b, 42);
        assert_eq!(*b.c, 42);
    }
}
