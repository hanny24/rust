// xfail-fast

// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[no_std];
extern mod std;
extern mod zed(name = "std");
extern mod bar(name = "std", vers = "0.7");


use std::str;
use x = zed::str;
mod baz {
    pub use bar::str;
    pub use x = std::str;
}

pub fn main() { }
