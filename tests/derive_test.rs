use core::cell::{Ref, RefCell};
use core::ptr::drop_in_place;

#[cfg(test)]
mod simple {
    use super::*;

    #[derive(incrstruct::IncrStruct)]
    struct AStruct<'a> {
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

    #[test]
    fn new_uninit_works() {
        let mut a = unsafe { AStruct::new_uninit(4711, RefCell::new(42)) };
        let aref = AStruct::ensure_init(&mut a);

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

        AStruct::force_init(&mut b);

        assert_eq!(*b.head1.borrow(), 42);
        assert_eq!(*b.b, 42);
        assert_eq!(*b.c, 42);
    }
}

#[cfg(test)]
mod init_err {
    use super::*;

    #[derive(Debug, Eq, PartialEq)]
    enum Error {
        Failed,
    }

    #[derive(Debug, incrstruct::IncrStruct)]
    #[init_err(Error)]
    struct AStruct<'a> {
        #[borrows(b, head2)]
        pub c: &'a i32,

        #[borrows(head1)]
        pub b: Ref<'a, i32>,

        pub head1: RefCell<i32>,
        pub head2: i64,

        #[header]
        hdr: incrstruct::Header,
    }

    const HEAD2_FAIL: i64 = 18;

    impl<'a> AStructInit<'a> for AStruct<'a> {
        fn init_field_c(head2: &'a i64, b: &'a Ref<'a, i32>) -> Result<&'a i32, Error> {
            if *head2 == HEAD2_FAIL {
                Err(Error::Failed)
            } else {
                Ok(b)
            }
        }

        fn init_field_b(head1: &'a RefCell<i32>) -> Result<Ref<'a, i32>, Error> {
            Ok(head1.borrow())
        }
    }

    #[test]
    fn new_uninit_works() {
        let mut a = unsafe { AStruct::new_uninit(4711, RefCell::new(42)) };
        let aref = AStruct::ensure_init(&mut a).unwrap();

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
        let a = AStruct::new_box(4711, RefCell::new(42)).unwrap();

        assert_eq!(*a.head1.borrow(), 42);
        assert_eq!(*a.b, 42);
        assert_eq!(*a.c, 42);
    }

    #[test]
    fn new_rc_works() {
        let a = AStruct::new_rc(4711, RefCell::new(42)).unwrap();

        assert_eq!(*a.head1.borrow(), 42);
        assert_eq!(*a.b, 42);
        assert_eq!(*a.c, 42);
    }

    #[test]
    fn force_init_works() {
        let a = AStruct::new_box(4711, RefCell::new(42)).unwrap();
        let mut b = *a;

        AStruct::force_init(&mut b).unwrap();

        assert_eq!(*b.head1.borrow(), 42);
        assert_eq!(*b.b, 42);
        assert_eq!(*b.c, 42);
    }

    #[test]
    fn new_box_fails_gracefully() {
        assert_eq!(
            AStruct::new_box(HEAD2_FAIL, RefCell::new(42)).unwrap_err(),
            Error::Failed
        );
    }

    #[test]
    fn second_force_init_fails_gracefully() {
        let a = AStruct::new_box(4711, RefCell::new(42)).unwrap();
        let mut b = *a;

        AStruct::force_init(&mut b).unwrap();

        b.head2 = HEAD2_FAIL;

        assert_eq!(AStruct::force_init(&mut b).unwrap_err(), Error::Failed);
    }
}
