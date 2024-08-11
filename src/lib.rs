//! Utilities for incrementally constructing self-referencing structs.
//!
//! Sometimes, it's useful for one field in a struct to reference
//! another. Rust makes this difficult, since structs are either
//! uninitialized or fully initialized. Many attempts have been made
//! to support this ubiquitous programming pattern, and Ouroboros is
//! probably the closest there is to a winner.
//!
//! It's a bit like computed properties in VueJS, computed observables
//! in KnockoutJS or effect hooks in React.
//!
//! # Status
//!
//! Alpha stage. You can probably shoot yourself in the foot with this
//! crate.
//!
//! [ ] Implement generics so it's not hard-coded for a single lifetime.
//!
//! # How To Define a Self-Referencing Struct
//!
//! Like Ouroboros, we divide struct fields into heads and tails. The
//! head fields are not referencing `self`, while the tail fields
//! do. In addition, you need to add a header field last:
//!
//! ```rust
//! use incrstruct::IncrStruct;
//!
//! #[derive(IncrStruct)]
//! struct AStruct<'a> {
//! #   _ph: std::marker::PhantomData<&'a ()>,
//!     #[header]
//!     hdr: incrstruct::Header,
//! }
//! # impl<'a> AStructInit<'a> for AStruct<'a> {}
//! ```
//!
//! The name of the field doesn't matter.
//!
//! Tail fields are decorated with `#[borrows()]`:
//!
//! ```rust
//! use std::cell::{Ref, RefCell};
//! # use incrstruct::IncrStruct;
//! # #[derive(IncrStruct)]
//! # struct AStruct<'a> {
//!     #[borrows(a)]
//!     b: Ref<'a, i32>,
//!
//!     a: RefCell<i32>,
//! #   #[header]
//! #   hdr: incrstruct::Header,
//! }
//! # impl<'a> AStructInit<'a> for AStruct<'a> {
//! #    fn init_field_b(a: &'a RefCell<i32>) -> Ref<'a, i32> {
//! #        todo!()
//! #    }
//! # }
//! ```
//!
//! Unlike Ouroboros, you can only borrow from fields later in the
//! struct (to enforce a sane drop order,) and only immutable
//! references are allowed.
//!
//! Lastly, you implement initialization functions in an
//! auto-generated trait, named like the struct with `Init`
//! appended. This trait is used any time you construct a new value,
//! or call `force_init` after moving:
//!
//! ```rust
//! # use std::cell::{Ref, RefCell};
//! # use incrstruct::IncrStruct;
//! # #[derive(IncrStruct)]
//! # struct AStruct<'a> {
//! #     #[borrows(a)]
//! #     b: Ref<'a, i32>,
//! #     a: RefCell<i32>,
//! #     #[header]
//! #     hdr: incrstruct::Header,
//! # }
//! impl<'a> AStructInit<'a> for AStruct<'a> {
//!     fn init_field_b(a: &'a RefCell<i32>) -> Ref<'a, i32> {
//!         a.borrow()
//!     }
//! }
//! ```
//!
//! # Creating A Value
//!
//! Now that `AStruct` is defined, we can easily create a `Box` or
//! `Rc` value:
//!
//! ```rust
//! # use std::cell::{Ref, RefCell};
//! # use incrstruct::IncrStruct;
//! # #[derive(IncrStruct)]
//! # struct AStruct<'a> {
//! #     #[borrows(a)]
//! #     b: Ref<'a, i32>,
//! #     a: RefCell<i32>,
//! #     #[header]
//! #     hdr: incrstruct::Header,
//! # }
//! # impl<'a> AStructInit<'a> for AStruct<'a> {
//! #    fn init_field_b(a: &'a RefCell<i32>) -> Ref<'a, i32> {
//! #        a.borrow()
//! #    }
//! # }
//! let my_box = AStruct::new_box(RefCell::new(42));
//! let my_rc = AStruct::new_rc(RefCell::new(42));
//!
//! assert_eq!(*my_box.a.borrow(), *my_box.b);
//! assert_eq!(*my_rc.a.borrow(), *my_rc.b);
//! ```
//!
//! These are generally safe, since you rarely move values out of
//! them. If you do move the value, the self-references will still be
//! pointing to the old place, so you need to run
//! `incrstruct::force_init`:
//!
//! ```rust
//! # use std::cell::{Ref, RefCell};
//! # use std::rc::Rc;
//! # use incrstruct::IncrStruct;
//! # #[derive(IncrStruct)]
//! # struct AStruct<'a> {
//! #     #[borrows(a)]
//! #     b: Ref<'a, i32>,
//! #     a: RefCell<i32>,
//! #     #[header]
//! #     hdr: incrstruct::Header,
//! # }
//! # impl<'a> AStructInit<'a> for AStruct<'a> {
//! #    fn init_field_b(a: &'a RefCell<i32>) -> Ref<'a, i32> {
//! #        a.borrow()
//! #    }
//! # }
//! let my_rc = AStruct::new_rc(RefCell::new(42));
//! let mut taken_value = Rc::into_inner(my_rc).unwrap();
//!
//! //assert_eq!(*taken_value.a.borrow(), *taken_value.b);  // UNSOUND!
//!
//! incrstruct::force_init(&mut taken_value);
//!
//! assert_eq!(*taken_value.a.borrow(), *taken_value.b);  // Good
//! ```
//!
//! If you really want to make a mess, you can use the low-level API,
//! which gives you control over each initialization phase
//! separately. This is useful e.g. in creating `Rc<RefCell<AStruct>>`
//! or other wrappers that aren't supported directly. Take a look at
//! the [new_box] function.
//!
//! # Example
//!
//! ```rust
//! use std::cell::{Ref, RefCell};
//! use incrstruct::IncrStruct;
//!
//! #[derive(IncrStruct)]
//! struct AStruct<'a> {
//!     #[borrows(b)]             // Borrowing from a tail field
//!     c: &'a Ref<'a, i32>,      // is possible.
//!
//!     #[borrows(a)]             // You can only borrow from fields that
//!     b: Ref<'a, i32>,          // come after the current field.
//!
//!     a: RefCell<i32>,          // A head field. Since you can only borrow
//!                               // immutable references, RefCell is useful.
//!
//!     #[header]                 // The required header field.
//!     hdr: incrstruct::Header,  // The name is arbitrary.
//! }
//!
//! // The AStructInit trait is generated by the derive macro and
//! // ensures the contract between the incrstruct library code and
//! // the user provided code matches. The functions are invoked in
//! // reverse field declaration order.
//! impl<'a> AStructInit<'a> for AStruct<'a> {
//!     fn init_field_c(b: &'a Ref<'a, i32>) -> &'a Ref<'a, i32> {
//!         b
//!     }
//!
//!     fn init_field_b(a: &'a RefCell<i32>) -> Ref<'a, i32> {
//!         a.borrow()
//!     }
//! }
//!
//! // Only head fields are provided to the generated `new_X` functions.
//! let my_a = AStruct::new_box(RefCell::new(42));
//!
//! assert_eq!(*my_a.a.borrow(), *my_a.b);
//! ```
//!
//! # How It Works
//!
//! The `IncrStruct` derive macro creates a two-phase initialization
//! scheme where `new_uninit` initializes all head fields, and `init`
//! initializes all tail fields. `init` is also called whenever
//! `force_init` is called.
//!
//! The header field keeps track of whether the tail fields have been
//! initialized or not. As long as the tail fields are invalid, we
//! prefer to reference `AStruct` as `MaybeUninit<AStruct>`, just to
//! make it obvious that you shouldn't use it yet.
//!
//! Aside from that, it simply calls the `init_field_X` functions in
//! order.
//!
//! A generated associated function called
//! `AStruct::drop_uninit_in_place` must be used to drop the
//! `MaybeUninit<AStruct>` if the second phase never runs. It will
//! panic if called on a fully initialized struct (but then you
//! shouldn't have a `MaybeUninit<AStruct>` reference to it anyway.)
//!
//! # Design Considerations
//!
//! If we narrow down the scope on self-referencing structs, we may be
//! able to find a better solution.
//!
//! * There is a strict DAG of dependencies between fields.
//! * The struct can be partially initialized, if it's clearly
//!   marked, e.g. with `MaybeUninit`.
//! * When a value is moved, the caller is responsible for
//!   re-initializing the dependent fields. Functions to do that
//!   are available.
//! * Constructor functions are idempotent, so they can be run
//!   whenever the value moves. This suggests using a trait rather
//!   than closures.
//! * A higher-level API can be used to make safe `new_box` and
//!   `new_rc` functions, making the need for unsafe functions
//!   and re-initialization limited in practice.
//!
//! And here is a wish list:
//!
//! * Don't `Box` individual field values. Use a derive macro, not
//!   rewriting what the user has defined. WYSIWYG.
//! * Moving an initialized struct is impossible. Moving partially
//!   initialized structs works.
//! * Initialization can fail, and `Results` are handled properly
//!   to drop already initialized fields.
//! * Since `&mut` is exclusive, it would be ideal if self-referential
//!   structs could only grab immutable references. (Since a single
//!   `&mut self` would imply that nothing else in the program can
//!   grab a reference. If, additionally, external users of the struct
//!   were unable to acquire a `&mut`, there would be no changes to
//!   Rust borrow semantics.
//! * Generics shouldn't be a problem.
//! * Enforce sound ordering of fields so that the natural drop order
//!   makes sense w.r.t. dependencies.

use core::marker::PhantomPinned;
use core::mem::MaybeUninit;
use core::ptr::drop_in_place;
use std::rc::Rc;

pub use incrstruct_derive::IncrStruct;

#[derive(Clone, Debug)]
pub enum Header {
    Uninited,
    Initing,
    Inited(PhantomPinned),
}

/// An trait implemented by all structures using incrstruct. The
/// implementation is auto-generated by the macros.
///
/// Used by auto-generated code. This is not an external API.
pub trait IncrStructInit: Sized {
    /// Initializes all leaf fields, in dependency order. All head
    /// fields have already been initialized, and all tail fields are
    /// uninitialized. When this function returns, all tail fields of
    /// the struct must have been initialized.
    unsafe fn init(this: *mut Self);

    /// Drops all tail fields. It is only called when `this` is fully
    /// initialized.
    unsafe fn drop_tail_in_place(this: &mut Self);

    /// Returns a reference to the incrstruct header. This field
    /// should be last, so it's dropped last.
    fn header<'b>(this: &'b mut Self) -> &'b mut Header;
}

/// Creates a `Box` from the given, partial struct. The function
/// initializes all fields. The input is normally created using
/// `T::new_uninit`.
///
/// Used by auto-generated code.
pub fn new_box<T: IncrStructInit>(v: MaybeUninit<T>) -> Box<T> {
    let bx = Box::new(v);
    let raw = Box::into_raw(bx);

    // SAFETY: we have taken ownership of the pointer to uninitialized Box data.
    let ptr = ensure_init(unsafe { &mut *raw });

    // SAFETY: the data is fully initialized, and Box can take ownership.
    unsafe { Box::from_raw(ptr as *mut _) }
}

/// Creates a `Rc` from the given, partial struct. The function
/// initializes all fields. The input is normally created using
/// `T::new_uninit`.
///
/// Used by auto-generated code.
pub fn new_rc<T: IncrStructInit>(v: MaybeUninit<T>) -> Rc<T> {
    let rc = Rc::new(v);
    let raw = Rc::into_raw(rc);

    // SAFETY: we have taken ownership of the pointer to
    // uninitialized Rc data. We are the only writers.
    let ptr = ensure_init(unsafe { &mut *(raw as *mut _) }) as *mut _;

    // SAFETY: the data is fully initialized, and Rc can take ownership.
    unsafe { Rc::from_raw(ptr) }
}

/// Creates a partially initialized struct. The `f` function
/// initializes all head fields, and only the head fields.
///
/// Used by auto-generated code.
///
/// # Safety
///
/// Calling this function yields a value that will not call
/// destructors. Callers must use `drop_uninit_in_place` until a
/// successful call to `ensure_init`. After `ensure_init`, the caller
/// owns the `*mut Self`, and normal drop rules apply.
pub unsafe fn new_uninit<T: IncrStructInit, F: FnOnce(&mut T)>(f: F) -> MaybeUninit<T> {
    let mut out = MaybeUninit::<T>::uninit();

    // SAFETY: we just created the uninitialized value.
    let this = unsafe { &mut *out.as_mut_ptr() };
    unsafe { core::ptr::write(<T as IncrStructInit>::header(this), Header::Uninited) };

    f(this);

    out
}

/// Finalizes a partially initialized struct. The returned reference
/// is guaranteed to be the same as `this`, and is only returned as a
/// type-safety convenience.
///
/// Used by auto-generated code.
pub fn ensure_init<T: IncrStructInit>(this: &mut MaybeUninit<T>) -> &mut T {
    // SAFETY: we have exclusive access to `this`.
    let r = unsafe { &mut *this.as_mut_ptr() };

    match <T as IncrStructInit>::header(r) {
        Header::Inited(_) => {}
        _ => force_init(r),
    };

    // SAFETY: all fields have been initialized.
    unsafe { this.assume_init_mut() }
}

/// Drops a partially initialized struct. Tail fields are assumed to
/// be uninitialized, while all head fields are assumed to be
/// initialized.
///
/// Used by auto-generated code.
pub fn drop_uninit_in_place<T: IncrStructInit, F: FnOnce(&mut T)>(mut this: MaybeUninit<T>, f: F) {
    // SAFETY: `this` was moved into here.
    let r = unsafe { &mut *this.as_mut_ptr() };

    match <T as IncrStructInit>::header(r) {
        Header::Uninited => {
            f(r);

            // SAFETY: we only drop head fields, and only once.
            unsafe { drop_in_place(<T as IncrStructInit>::header(r)) };
        }
        Header::Inited(_) => panic!("drop_uninit_in_place on initialized value"),
        Header::Initing => panic!("drop_uninit_in_place during initialization"),
    }
}

/// Forces initialization of `this`, even if it was previously initialized.
///
/// This is useful when a T has moved, and the self-referencing tail
/// fields must be synchronized.
pub fn force_init<T: IncrStructInit>(this: &mut T) {
    match <T as IncrStructInit>::header(this) {
        Header::Uninited => {}
        // SAFETY: we are now making `this` back into a partially
        // initialized struct, the same as Uninited.
        Header::Inited(_) => unsafe {
            T::drop_tail_in_place(this);
        },
        Header::Initing => panic!("Recursive call to force_init"),
    };

    *<T as IncrStructInit>::header(this) = Header::Initing;

    // If we panic in the middle of init(), data will
    // leak without being dropped, even if
    // drop_uninit_in_place is invoked later.
    //
    // It seems most things in the Rust standard library
    // are not unwind-safe, e.g. unlocking mutexes on
    // panic.
    //
    // SAFETY: the code above has made the struct partially
    // initialized.

    unsafe { T::init(this) };

    *<T as IncrStructInit>::header(this) = Header::Inited(PhantomPinned);
}
