#![no_std]
extern crate runtime as std;

use std::ktest;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[ktest]
mod tests {
    use super::*;

    #[ktest]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[ktest]
    #[should_panic]
    fn it_fails() {
        let result = add(2, 2);
        assert_eq!(result, 5);
    }
}
