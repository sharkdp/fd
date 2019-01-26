// Copyright (c) 2019 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

// inspired by https://www.reddit.com/r/rust/comments/9406rl/once_cell_a_lazy_static_without_macros_and_more/

// There are several crates out there dealing with lazy initialization
// in different ways. Pretty much all seem to use some form of additional
// locking or atomic operations, or other kinds of overhead.
//
// For this usecase, what was needed was a low-overhead solution,
// that only needed to support a single thread.
//
// TODO: maybe this should be polished/expanded and pulled out in a crate ?

pub struct Lazy<T, F>
where
    F: FnOnce() -> T,
{
    inner: LazyImpl<T, F>,
}

impl<T, F> Lazy<T, F>
where
    F: FnOnce() -> T,
{
    pub fn new(f: F) -> Self {
        Self {
            inner: LazyImpl::Pending(Some(f)),
        }
    }

    pub fn get(&mut self) -> &T {
        if let LazyImpl::Pending(ref mut obtain) = self.inner {
            let obtain = obtain.take().unwrap();
            self.inner = LazyImpl::Data(obtain());
        }
        match self.inner {
            LazyImpl::Data(ref res) => res,
            _ => panic!(),
        }
    }
}

enum LazyImpl<T, F>
where
    F: FnOnce() -> T,
{
    Pending(Option<F>),
    Data(T),
}

#[test]
fn evaluates_only_once() {
    let mut x = 2;
    let mut l = Lazy::new(|| {
        x += 1;
        x
    });

    assert_eq!(*l.get(), 3);
    assert_eq!(*l.get(), 3);
}

#[test]
fn no_evaluation_before_get() {
    #[allow(unreachable_code)]
    let _l = Lazy::new(|| {
        panic!("must not call this");
        3
    });
}
