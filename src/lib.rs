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
//! - Since a move in Rust doesn't trigger any code, any value move
//!   will make the self-referencing struct invalid. E.g. if you use
//!   these structs directly in a `Vec`, which later has to reallocate
//!   to grow. As long as you use `Vec<Pin<Box<MyStruct>>>`, like what the
//!   high-level API provides with e.g. [new_box], it is safe.
//!    - We could provide a special `Vec` (and other in-line containers)
//!      that runs `force_init`. It could be eager or lazy, though the
//!      lazy case would be complicated by borrowed slices. This would
//!      clean up the heap usage (and cache utilization) further.
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
//! You will likely always want a lifetime parameter, so you can refer
//! back to it in tail fields. The first declared lifetime parameter
//! is used for the `init_field_myfield` arguments.
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
//! # How To Create A Value
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
//! #     fn init_field_b(a: &'a RefCell<i32>) -> Ref<'a, i32> {
//! #         a.borrow()
//! #     }
//! # }
//! let my_box = AStruct::new_box(RefCell::new(42));
//! let my_rc = AStruct::new_rc(RefCell::new(42));
//!
//! assert_eq!(*my_box.a.borrow(), *my_box.b);
//! assert_eq!(*my_rc.a.borrow(), *my_rc.b);
//! ```
//!
//! These are safe, since they return a
//! [`Pin<>`](https://doc.rust-lang.org/std/pin/struct.Pin.html), and
//! thus the value cannot be moved. If you do move the value (using
//! unsafe code,) the self-references will still be pointing to the
//! old place, so you need to run `AStruct::force_init`:
//!
//! ```rust
//! use core::pin::Pin;
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
//! #     fn init_field_b(a: &'a RefCell<i32>) -> Ref<'a, i32> {
//! #         a.borrow()
//! #     }
//! # }
//!
//! let my_rc = AStruct::new_rc(RefCell::new(42));
//! let mut taken_value = Rc::into_inner(unsafe { Pin::into_inner_unchecked(my_rc) }).unwrap();
//!
//! //assert_eq!(*taken_value.a.borrow(), *taken_value.b);  // UNSOUND!
//!
//! AStruct::force_init(&mut taken_value);
//!
//! assert_eq!(*taken_value.a.borrow(), *taken_value.b);  // Good
//! ```
//!
//! If you really want to make a mess, you can use the low-level API,
//! which gives you control over each initialization phase
//! separately. This is useful e.g. in creating
//! `Pin<Rc<RefCell<AStruct>>>` or other wrappers that aren't
//! supported directly. Take a look at the [new_box] function. Note
//! that you must always use `Pin<>` to wrap your smart pointer,
//! ensuring the value cannot be moved by Rust.
//!
//! # Handling Failures
//!
//! Using the `#[init_err(AnError)]` attribute on the struct, the
//! `init_field_myfield` functions are expected to return a `Result<T,
//! AnError>` instead of the plain value. If any tail field
//! initialization fails, the previously initialized tail fields are
//! dropped before `ensure_init` returns. This also causes relevant
//! generated functions on your struct to return a corresponding
//! `Result`:
//!
//! - `new_box -> Result<Pin<Box<AStruct>>, AnError>`
//! - `new_rc -> Result<Pin<Rc<AStruct>>, AnError>`
//! - `ensure_init -> Result<&mut AStruct, AnError>`
//! - `force_init -> Result<(), AnError>`
//!
//! Note that Rust is generally not panic-tolerant, and no attempts to
//! drop are made if a field initialization function panics.
//!
//! If you are using the unsafe `new_uninit`, and `ensure_init` fails,
//! remember to run `drop_uninit` to stop memory leaks.
//!
//! # Examples
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
//! ## Generics And Lifetimes
//!
//! Generic parameters are forwarded to the generated Init trait.
//!
//! The struct's first declared lifetime is used to set the lifetime
//! of the argument references in `init_field_myfield`.
//!
//! ```rust
//! use std::cell::{Ref, RefCell};
//! use std::fmt::Debug;
//! use incrstruct::IncrStruct;
//!
//! #[derive(IncrStruct)]
//! struct AStruct<'b, T> where T: Debug {
//!     #[borrows(a)]
//!     b: Ref<'b, T>,
//!     a: RefCell<T>,
//!
//!     #[header]
//!     hdr: incrstruct::Header,
//! }
//!
//! impl<'b, T: Debug> AStructInit<'b, T> for AStruct<'b, T> {
//!     fn init_field_b(a: &'b RefCell<T>) -> Ref<'b, T> {
//!         a.borrow()
//!     }
//! }
//!
//! let my_box = AStruct::new_box(RefCell::new(42));
//!
//! assert_eq!(*my_box.a.borrow(), *my_box.b);
//! ```
//!
//! ## Handling Failures
//!
//! ```rust
//! use std::cell::{Ref, RefCell};
//! use incrstruct::IncrStruct;
//!
//! #[derive(Debug, Eq, PartialEq)]
//! enum AnError {
//!   Failed,
//! }
//!
//! #[derive(Debug, IncrStruct)]
//! #[init_err(AnError)]
//! struct AStruct<'a> {
//!     #[borrows(a)]
//!     b: Ref<'a, i32>,
//!
//!     a: RefCell<i32>,
//!
//!     #[header]
//!     hdr: incrstruct::Header,
//! }
//!
//! // All functions must return a `Result<_, AnError>`.
//! impl<'a> AStructInit<'a> for AStruct<'a> {
//!     fn init_field_b(a: &'a RefCell<i32>) -> Result<Ref<'a, i32>, AnError> {
//!         Err(AnError::Failed)
//!     }
//! }
//!
//! let result = AStruct::new_box(RefCell::new(42));
//!
//! assert_eq!(result.unwrap_err(), AnError::Failed);
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
//! Aside from that, it simply calls the `init_field_myfield`
//! functions in order.
//!
//! To support initialization functions that can fail, the generated
//! `init` function keeps track of which field it is initializing, and
//! calls the generated `drop_tail_in_place` for the previous
//! ones. There is no concept of partially initialized tail fields;
//! it's all or nothing after the second phase returns.
//!
//! A generated associated function called `AStruct::drop_uninit` must
//! be used to drop the `MaybeUninit<AStruct>` if the second phase
//! never runs. It will panic if called on a fully initialized struct
//! (but then you shouldn't have a `MaybeUninit<AStruct>` reference to
//! it anyway.)
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
//! * [x] Don't `Box` individual field values. Use a derive macro, not
//!       rewriting what the user has defined. WYSIWYG.
//! * [x] Initialization can fail, and `Results` are handled properly
//!       to drop already initialized fields.
//! * [x] Generics shouldn't be a problem.
//! * [x] Enforce sound ordering of fields so that the natural drop order
//!       makes sense w.r.t. dependencies.
//! * [x] Moving an initialized struct is impossible. Moving partially
//!       initialized structs works.
//! * [ ] Since `&mut` is exclusive, it would be ideal if self-referential
//!       structs could only grab immutable references. (Since a single
//!       `&mut self` would imply that nothing else in the program can
//!       grab a reference. If, additionally, external users of the struct
//!       were unable to acquire a `&mut`, there would be no changes to
//!       Rust borrow semantics.

use core::marker::PhantomPinned;

pub use incrstruct_derive::IncrStruct;

pub mod internal;

#[derive(Clone, Debug)]
pub enum Header {
    // All head fields are initialized, and no tail fields are. The
    // struct is wrapped in `MaybeUninit<>` and `drop_uninit` must be
    // called manually to drop head fields.
    Uninited,

    // All head fields are initialized, and some tail fields may be
    // initialized. This is used to check for undue recursive calls to
    // `ensure_init`.
    Initing,

    // All fields are initialized, and the struct is not allowed to
    // move. (Where in the enum the `PhantomPinned` is located doesn't
    // matter, but it only matters for this variant.)
    Inited(PhantomPinned),
}
