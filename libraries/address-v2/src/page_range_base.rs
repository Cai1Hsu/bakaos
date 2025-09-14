macro_rules! impl_page_range {
    ($page_range_type:ident, $page_type:ident, $addr_type:ty, $range_type:ty, $(#[$doc:meta])*) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $page_range_type {
            start: $page_type,
            page_count: usize
        }

        impl $page_range_type {
            /// Creates a new page range starting at `start` and covering `page_count` pages.
            ///
            /// # Parameters
            /// - `start`: The starting page of the range (inclusive)
            /// - `page_count`: The number of pages in the range
            ///
            /// # Returns
            /// A new page range spanning the specified number of pages
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range = PhysPageRange::new(start_page, 3);
            /// assert_eq!(range.len(), 3);
            /// ```
            pub const fn new(start: $page_type, page_count: usize) -> Self {
                Self {
                    start,
                    page_count,
                }
            }

            /// Creates a new page range from a start page to an end page (exclusive).
            ///
            /// This function validates that the start and end pages have the same size
            /// and that the start address is not greater than the end address.
            ///
            /// # Parameters
            /// - `start`: The starting page of the range (inclusive)
            /// - `end`: The ending page of the range (exclusive)
            ///
            /// # Returns
            /// - `Some(range)` if the pages are compatible and properly ordered
            /// - `None` if the pages have different sizes or are improperly ordered
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let end = PhysPage::new_4k(PhysAddr::new(0x3000)).unwrap();
            /// let range = PhysPageRange::from_start_end(start, end);
            /// assert!(range.is_some());
            /// assert_eq!(range.unwrap().len(), 2);
            ///
            /// // Different page sizes fail
            /// let start_4k = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let end_2m = PhysPage::new_2m(PhysAddr::new(0x200000)).unwrap();
            /// let invalid = PhysPageRange::from_start_end(start_4k, end_2m);
            /// assert!(invalid.is_none());
            /// ```
            pub const fn from_start_end(start: $page_type, end: $page_type) -> Option<Self> {
                if start.size() != end.size() || *start.addr() > *end.addr() {
                    return None;
                }

                // This should be guranteed by the page type alignment
                debug_assert!((*end.addr() - *start.addr()).is_multiple_of(start.size()));

                Some(Self {
                    start,
                    page_count: ((end.addr() - start.addr()) as usize) / start.size(),
                })
            }
        }

        impl $page_range_type {
            /// Returns the starting page of the range.
            ///
            /// # Returns
            /// The first page in the range (inclusive)
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range = PhysPageRange::new(start_page, 2);
            /// assert_eq!(range.start(), start_page);
            /// ```
            #[inline(always)]
            pub const fn start(&self) -> $page_type {
                self.start
            }

            /// Returns the ending page of the range (exclusive).
            ///
            /// # Returns
            /// The page immediately after the last page in the range
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range = PhysPageRange::new(start_page, 2);
            /// let expected_end = start_page + 2;
            /// assert_eq!(range.end(), expected_end);
            /// ```
            #[inline(always)]
            pub const fn end(&self) -> $page_type {
                self.start + self.page_count
            }

            /// Calculates the total length of the range in bytes.
            ///
            /// # Returns
            /// The total number of bytes covered by all pages in the range
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range = PhysPageRange::new(start_page, 3);
            /// assert_eq!(range.addr_len(), 3 * 0x1000); // 3 * 4KB = 12KB
            /// ```
            #[inline(always)]
            pub const fn addr_len(&self) -> usize {
                self.len() * self.start.size()
            }

            /// Returns the number of pages in the range.
            ///
            /// # Returns
            /// The count of pages in this range
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range = PhysPageRange::new(start_page, 5);
            /// assert_eq!(range.len(), 5);
            /// ```
            #[inline(always)]
            pub const fn len(&self) -> usize {
                self.page_count
            }

            /// Determines if the range is empty (contains zero pages).
            ///
            /// # Returns
            /// `true` if the range contains no pages, `false` otherwise
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let empty_range = PhysPageRange::new(start_page, 0);
            /// assert!(empty_range.is_empty());
            ///
            /// let non_empty_range = PhysPageRange::new(start_page, 1);
            /// assert!(!non_empty_range.is_empty());
            /// ```
            #[inline(always)]
            pub const fn is_empty(&self) -> bool {
                self.addr_len() == 0
            }

            /// Converts this page range to an address range.
            ///
            /// # Returns
            /// An address range covering the same memory area as this page range
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let page_range = PhysPageRange::new(start_page, 2);
            /// let addr_range = page_range.as_addr_range();
            /// assert_eq!(addr_range.start(), PhysAddr::new(0x1000));
            /// assert_eq!(addr_range.end(), PhysAddr::new(0x3000));
            /// ```
            #[inline(always)]
            pub const fn as_addr_range(&self) -> $range_type {
                <$range_type>::new(self.start().addr(), self.end().addr())
            }

            /// Checks if this range contains a specific page.
            ///
            /// The page must have the same size as the pages in this range.
            ///
            /// # Parameters
            /// - `page`: The page to check for containment
            ///
            /// # Returns
            /// `true` if the page is contained within this range, `false` otherwise
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range = PhysPageRange::new(start_page, 3);
            ///
            /// let contained_page = PhysPage::new_4k(PhysAddr::new(0x2000)).unwrap();
            /// assert!(range.contains(contained_page));
            ///
            /// let outside_page = PhysPage::new_4k(PhysAddr::new(0x4000)).unwrap();
            /// assert!(!range.contains(outside_page));
            /// ```
            pub fn contains(&self, page: $page_type) -> bool {
                // Pages must have the same size to be comparable
                debug_assert!(self.start.size() == page.size());

                let start_addr = *self.start().addr();
                let end_addr = *self.end().addr();
                let page_addr = *page.addr();

                start_addr <= page_addr && page_addr < end_addr
            }

            /// Checks if this range completely contains another range.
            ///
            /// Both ranges must have compatible page sizes for comparison.
            ///
            /// # Parameters
            /// - `other`: The range to check for containment
            ///
            /// # Returns
            /// `true` if `other` is completely contained within this range, `false` otherwise
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start1 = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range1 = PhysPageRange::new(start1, 4); // 0x1000..0x5000
            ///
            /// let start2 = PhysPage::new_4k(PhysAddr::new(0x2000)).unwrap();
            /// let range2 = PhysPageRange::new(start2, 2); // 0x2000..0x4000
            ///
            /// assert!(range1.contains_range(range2));
            /// assert!(!range2.contains_range(range1));
            /// ```
            pub fn contains_range(&self, other: Self) -> bool {
                // Ranges must have compatible page sizes
                debug_assert!(self.start.size() == other.start.size());

                let self_start = *self.start().addr();
                let self_end = *self.end().addr();
                let other_start = *other.start().addr();
                let other_end = *other.end().addr();

                self_start <= other_start && other_end <= self_end
            }

            /// Checks if this range is completely contained by another range.
            ///
            /// # Parameters
            /// - `other`: The range that might contain this range
            ///
            /// # Returns
            /// `true` if this range is completely contained within `other`, `false` otherwise
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start1 = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let small_range = PhysPageRange::new(start1, 2);
            ///
            /// let start2 = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let large_range = PhysPageRange::new(start2, 4);
            ///
            /// assert!(small_range.contained_by(large_range));
            /// ```
            pub fn contained_by(&self, other: Self) -> bool {
                other.contains_range(*self)
            }

            /// Checks if this range intersects with another range.
            ///
            /// Two ranges intersect if they have any overlapping pages.
            /// Both ranges must have compatible page sizes.
            ///
            /// # Parameters
            /// - `other`: The range to check for intersection
            ///
            /// # Returns
            /// `true` if the ranges intersect, `false` otherwise
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start1 = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range1 = PhysPageRange::new(start1, 3); // 0x1000..0x4000
            ///
            /// let start2 = PhysPage::new_4k(PhysAddr::new(0x3000)).unwrap();
            /// let range2 = PhysPageRange::new(start2, 2); // 0x3000..0x5000
            ///
            /// assert!(range1.intersects(range2));
            /// ```
            pub fn intersects(&self, other: Self) -> bool {
                // Ranges must have compatible page sizes
                debug_assert!(self.start.size() == other.start.size());

                let self_start = *self.start().addr();
                let self_end = *self.end().addr();
                let other_start = *other.start().addr();
                let other_end = *other.end().addr();

                self_start < other_end && other_start < self_end
            }

            /// Calculates the intersection of this range with another range.
            ///
            /// Returns the overlapping portion of two ranges, or None if they don't intersect
            /// or have incompatible page sizes.
            ///
            /// # Parameters
            /// - `other`: The range to intersect with
            ///
            /// # Returns
            /// - `Some(range)` containing the overlapping pages if ranges intersect
            /// - `None` if ranges don't intersect or have incompatible page sizes
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start1 = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range1 = PhysPageRange::new(start1, 3); // 0x1000..0x4000
            ///
            /// let start2 = PhysPage::new_4k(PhysAddr::new(0x2000)).unwrap();
            /// let range2 = PhysPageRange::new(start2, 3); // 0x2000..0x5000
            ///
            /// let intersection = range1.intersection(range2).unwrap();
            /// assert_eq!(intersection.start().addr(), PhysAddr::new(0x2000));
            /// assert_eq!(intersection.end().addr(), PhysAddr::new(0x4000));
            /// ```
            pub fn intersection(&self, other: Self) -> Option<Self> {
                if !self.intersects(other) {
                    return None;
                }

                let self_start = *self.start().addr();
                let self_end = *self.end().addr();
                let other_start = *other.start().addr();
                let other_end = *other.end().addr();

                let start_addr = core::cmp::max(self_start, other_start);
                let end_addr = core::cmp::min(self_end, other_end);

                // Create new pages from the intersection addresses
                let start_page = $page_type::new_custom_unchecked(
                    <$addr_type>::new(start_addr),
                    self.start.size()
                );
                let end_page = $page_type::new_custom_unchecked(
                    <$addr_type>::new(end_addr),
                    self.start.size()
                );

                Self::from_start_end(start_page, end_page)
            }

            /// Calculates the union of this range with another range.
            ///
            /// Creates the smallest range that contains both input ranges.
            ///
            /// # Parameters
            /// - `other`: The range to union with
            ///
            /// # Returns
            /// - `Some(range)` containing the smallest range that encompasses both inputs
            /// - `None` if the page sizes are incompatible
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start1 = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range1 = PhysPageRange::new(start1, 2); // 0x1000..0x3000
            ///
            /// let start2 = PhysPage::new_4k(PhysAddr::new(0x4000)).unwrap();
            /// let range2 = PhysPageRange::new(start2, 2); // 0x4000..0x6000
            ///
            /// let union = range1.union(range2).unwrap();
            /// assert_eq!(union.start().addr(), PhysAddr::new(0x1000));
            /// assert_eq!(union.end().addr(), PhysAddr::new(0x6000));
            /// ```
            pub fn union(&self, other: Self) -> Option<Self> {
                // Ranges must have compatible page sizes
                debug_assert!(self.start.size() == other.start.size());


                let self_start = *self.start().addr();
                let self_end = *self.end().addr();
                let other_start = *other.start().addr();
                let other_end = *other.end().addr();

                let start_addr = core::cmp::min(self_start, other_start);
                let end_addr = core::cmp::max(self_end, other_end);

                // Create new pages from the union addresses
                let start_page = $page_type::new_custom_unchecked(
                    <$addr_type>::new(start_addr),
                    self.start.size()
                );
                let end_page = $page_type::new_custom_unchecked(
                    <$addr_type>::new(end_addr),
                    self.start.size()
                );

                Self::from_start_end(start_page, end_page)
            }

            /// Offsets this range by a signed amount in bytes.
            ///
            /// Shifts the entire range by the specified byte offset. The range maintains
            /// its size but starts at a new address.
            ///
            /// # Parameters
            /// - `offset`: The byte offset to apply (can be positive or negative)
            ///
            /// # Returns
            /// - `Some(range)` with the new offset if the result is valid
            /// - `None` if the offset would cause address underflow
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x2000)).unwrap();
            /// let range = PhysPageRange::new(start_page, 2);
            ///
            /// // Positive offset
            /// let shifted_up = range.off_by(0x1000).unwrap();
            /// assert_eq!(shifted_up.start().addr(), PhysAddr::new(0x3000));
            ///
            /// // Negative offset
            /// let shifted_down = range.off_by(-0x1000isize).unwrap();
            /// assert_eq!(shifted_down.start().addr(), PhysAddr::new(0x1000));
            ///
            /// // Underflow returns None
            /// let underflow = range.off_by(-0x3000isize);
            /// assert!(underflow.is_none());
            /// ```
            pub fn off_by(&self, offset: isize) -> Option<Self> {
                // Calculate the new start address
                let new_start_addr = *self.start().addr() as isize + offset;

                // Check for underflow
                if new_start_addr < 0 {
                    return None;
                }

                let new_start_page = $page_type::new_custom_unchecked(
                    <$addr_type>::new(new_start_addr as usize),
                    self.start.size()
                );

                Some(Self::new(new_start_page, self.page_count))
            }
        }

        impl ::core::fmt::Display for $page_range_type {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(
                    f,
                    "{}({} pages, {:#x}..{:#x})",
                    stringify!($page_range_type),
                    self.len(),
                    *self.start().addr(),
                    *self.end().addr()
                )
            }
        }

        impl $page_range_type {
            /// Creates an iterator over all pages in this range.
            ///
            /// The iterator yields each page in the range in order from start to end.
            ///
            /// # Returns
            /// An iterator that yields each page in the range
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range = PhysPageRange::new(start_page, 3);
            ///
            /// let pages: Vec<_> = range.iter().collect();
            /// assert_eq!(pages.len(), 3);
            /// assert_eq!(pages[0].addr(), PhysAddr::new(0x1000));
            /// assert_eq!(pages[1].addr(), PhysAddr::new(0x2000));
            /// assert_eq!(pages[2].addr(), PhysAddr::new(0x3000));
            /// ```
            pub fn iter(&self) -> RangeIterator {
                RangeIterator {
                    current: self.start,
                    end: self.end(),
                }
            }
        }

        impl IntoIterator for $page_range_type {
            type Item = $page_type;
            type IntoIter = RangeIterator;

            /// Converts the page range into an iterator.
            ///
            /// This allows using `for` loops and other iterator-based operations
            /// directly on page ranges.
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysPageRange, PhysAddr};
            /// let start_page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range = PhysPageRange::new(start_page, 2);
            ///
            /// for (i, page) in range.into_iter().enumerate() {
            ///     println!("Page {}: {:?}", i, page);
            /// }
            /// ```
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        /// Iterator for page ranges.
        ///
        /// This iterator yields each page in a page range in sequential order.
        /// It's created by calling `iter()` on a page range or by using the
        /// `IntoIterator` implementation.
        #[derive(Debug, Clone)]
        pub struct RangeIterator {
            current: $page_type,
            end: $page_type,
        }

        impl Iterator for RangeIterator {
            type Item = $page_type;

            /// Returns the next page in the range.
            ///
            /// # Returns
            /// - `Some(page)` if there are more pages in the range
            /// - `None` if the iterator has reached the end
            fn next(&mut self) -> Option<Self::Item> {
                // Compare using the underlying addresses
                if *self.current.addr() < *self.end.addr() {
                    let current = self.current;
                    self.current += 1;
                    Some(current)
                } else {
                    None
                }
            }
        }

        #[cfg(test)]
        mod page_range_tests {
            use super::*;

            /// Test basic page range creation
            #[test]
            fn test_page_range_new() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range = $page_range_type::new(start_page, 3);

                assert_eq!(range.start(), start_page);
                assert_eq!(range.len(), 3);
                assert_eq!(range.addr_len(), 3 * 0x1000);
                assert!(!range.is_empty());
            }

            #[test]
            fn test_page_range_new_zero_pages() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range = $page_range_type::new(start_page, 0);

                assert_eq!(range.len(), 0);
                assert_eq!(range.addr_len(), 0);
                assert!(range.is_empty());
            }

            #[test]
            fn test_page_range_from_start_end() {
                let start = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let end = $page_type::new_4k(<$addr_type>::new(0x3000)).unwrap();
                let range = $page_range_type::from_start_end(start, end);

                assert!(range.is_some());
                let range = range.unwrap();
                assert_eq!(range.len(), 2);
                assert_eq!(range.start(), start);
                assert_eq!(*range.end().addr(), 0x3000);
            }

            #[test]
            fn test_page_range_from_start_end_invalid() {
                // Different page sizes
                let start_4k = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let end_2m = $page_type::new_2m(<$addr_type>::new(0x200000)).unwrap();
                let invalid_range = $page_range_type::from_start_end(start_4k, end_2m);
                assert!(invalid_range.is_none());

                // Start after end
                let start = $page_type::new_4k(<$addr_type>::new(0x3000)).unwrap();
                let end = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let invalid_range = $page_range_type::from_start_end(start, end);
                assert!(invalid_range.is_none());
            }

            #[test]
            fn test_page_range_accessors() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x2000)).unwrap();
                let range = $page_range_type::new(start_page, 4);

                assert_eq!(range.start(), start_page);
                assert_eq!(*range.end().addr(), 0x6000);
                assert_eq!(range.len(), 4);
                assert_eq!(range.addr_len(), 4 * 0x1000);
                assert!(!range.is_empty());
            }

            #[test]
            fn test_page_range_as_addr_range() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let page_range = $page_range_type::new(start_page, 2);
                let addr_range = page_range.as_addr_range();

                assert_eq!(*addr_range.start(), 0x1000);
                assert_eq!(*addr_range.end(), 0x3000);
            }

            #[test]
            fn test_page_range_contains_page() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range = $page_range_type::new(start_page, 3); // 0x1000..0x4000

                // Page within range
                let contained_page = $page_type::new_4k(<$addr_type>::new(0x2000)).unwrap();
                assert!(range.contains(contained_page));

                // Page at start boundary
                let start_boundary = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                assert!(range.contains(start_boundary));

                // Page at end boundary (exclusive)
                let end_boundary = $page_type::new_4k(<$addr_type>::new(0x4000)).unwrap();
                assert!(!range.contains(end_boundary));

                // Page outside range
                let outside_page = $page_type::new_4k(<$addr_type>::new(0x5000)).unwrap();
                assert!(!range.contains(outside_page));
            }

            #[test]
            fn test_page_range_contains_range() {
                let start1 = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let large_range = $page_range_type::new(start1, 6); // 0x1000..0x7000

                let start2 = $page_type::new_4k(<$addr_type>::new(0x2000)).unwrap();
                let small_range = $page_range_type::new(start2, 2); // 0x2000..0x4000

                assert!(large_range.contains_range(small_range));
                assert!(!small_range.contains_range(large_range));

                // Same range contains itself
                assert!(large_range.contains_range(large_range));

                // Partially overlapping ranges
                let start3 = $page_type::new_4k(<$addr_type>::new(0x6000)).unwrap();
                let overlap_range = $page_range_type::new(start3, 3); // 0x6000..0x9000
                assert!(!large_range.contains_range(overlap_range));
                assert!(!overlap_range.contains_range(large_range));
            }

            #[test]
            fn test_page_range_contained_by() {
                let start1 = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let small_range = $page_range_type::new(start1, 2);

                let start2 = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let large_range = $page_range_type::new(start2, 4);

                assert!(small_range.contained_by(large_range));
                assert!(!large_range.contained_by(small_range));
            }

            #[test]
            fn test_page_range_intersects() {
                let start1 = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range1 = $page_range_type::new(start1, 3); // 0x1000..0x4000

                let start2 = $page_type::new_4k(<$addr_type>::new(0x3000)).unwrap();
                let range2 = $page_range_type::new(start2, 2); // 0x3000..0x5000

                assert!(range1.intersects(range2));
                assert!(range2.intersects(range1));

                // Non-intersecting ranges
                let start3 = $page_type::new_4k(<$addr_type>::new(0x5000)).unwrap();
                let range3 = $page_range_type::new(start3, 2); // 0x5000..0x7000
                assert!(!range1.intersects(range3));
                assert!(!range3.intersects(range1));

                // Adjacent ranges don't intersect
                let start4 = $page_type::new_4k(<$addr_type>::new(0x4000)).unwrap();
                let range4 = $page_range_type::new(start4, 1); // 0x4000..0x5000
                assert!(!range1.intersects(range4));
            }

            #[test]
            fn test_page_range_intersection() {
                let start1 = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range1 = $page_range_type::new(start1, 4); // 0x1000..0x5000

                let start2 = $page_type::new_4k(<$addr_type>::new(0x3000)).unwrap();
                let range2 = $page_range_type::new(start2, 3); // 0x3000..0x6000

                let intersection = range1.intersection(range2);
                assert!(intersection.is_some());

                let intersection = intersection.unwrap();
                assert_eq!(*intersection.start().addr(), 0x3000);
                assert_eq!(*intersection.end().addr(), 0x5000);
                assert_eq!(intersection.len(), 2);

                // Non-intersecting ranges
                let start3 = $page_type::new_4k(<$addr_type>::new(0x6000)).unwrap();
                let range3 = $page_range_type::new(start3, 2);
                let no_intersection = range1.intersection(range3);
                assert!(no_intersection.is_none());
            }

            #[test]
            fn test_page_range_union() {
                let start1 = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range1 = $page_range_type::new(start1, 2); // 0x1000..0x3000

                let start2 = $page_type::new_4k(<$addr_type>::new(0x4000)).unwrap();
                let range2 = $page_range_type::new(start2, 2); // 0x4000..0x6000

                let union = range1.union(range2);
                assert!(union.is_some());

                let union = union.unwrap();
                assert_eq!(*union.start().addr(), 0x1000);
                assert_eq!(*union.end().addr(), 0x6000);
                assert_eq!(union.len(), 5); // spans 0x1000..0x6000 = 5 pages

                // Union with overlapping ranges
                let start3 = $page_type::new_4k(<$addr_type>::new(0x2000)).unwrap();
                let range3 = $page_range_type::new(start3, 3); // 0x2000..0x5000

                let overlapping_union = range1.union(range3);
                assert!(overlapping_union.is_some());

                let overlapping_union = overlapping_union.unwrap();
                assert_eq!(*overlapping_union.start().addr(), 0x1000);
                assert_eq!(*overlapping_union.end().addr(), 0x5000);
            }

            #[test]
            fn test_page_range_off_by() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x2000)).unwrap();
                let range = $page_range_type::new(start_page, 2);

                // Positive offset
                let shifted_up = range.off_by(0x1000);
                assert!(shifted_up.is_some());
                let shifted_up = shifted_up.unwrap();
                assert_eq!(*shifted_up.start().addr(), 0x3000);
                assert_eq!(shifted_up.len(), 2);

                // Negative offset
                let shifted_down = range.off_by(-0x1000);
                assert!(shifted_down.is_some());
                let shifted_down = shifted_down.unwrap();
                assert_eq!(*shifted_down.start().addr(), 0x1000);
                assert_eq!(shifted_down.len(), 2);

                // Zero offset
                let no_shift = range.off_by(0);
                assert!(no_shift.is_some());
                let no_shift = no_shift.unwrap();
                assert_eq!(no_shift.start(), range.start());

                // Underflow
                let underflow = range.off_by(-0x3000);
                assert!(underflow.is_none());
            }

            #[test]
            fn test_page_range_iterator() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range = $page_range_type::new(start_page, 3);

                let pages: Vec<_> = range.iter().collect();
                assert_eq!(pages.len(), 3);

                assert_eq!(*pages[0].addr(), 0x1000);
                assert_eq!(*pages[1].addr(), 0x2000);
                assert_eq!(*pages[2].addr(), 0x3000);

                // Test that all pages have the same size
                for page in &pages {
                    assert_eq!(page.size(), 0x1000);
                }
            }

            #[test]
            fn test_page_range_into_iterator() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range = $page_range_type::new(start_page, 2);

                let mut count = 0;
                let mut expected_addr = 0x1000;

                for page in range {
                    assert_eq!(*page.addr(), expected_addr);
                    expected_addr += 0x1000;
                    count += 1;
                }

                assert_eq!(count, 2);
            }

            #[test]
            fn test_empty_range_iterator() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let empty_range = $page_range_type::new(start_page, 0);

                let pages: Vec<_> = empty_range.iter().collect();
                assert_eq!(pages.len(), 0);
            }

            #[test]
            fn test_single_page_range() {
                let page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range = $page_range_type::new(page, 1);

                assert_eq!(range.len(), 1);
                assert_eq!(range.addr_len(), 0x1000);
                assert!(!range.is_empty());

                assert!(range.contains(page));
                assert_eq!(range.start(), page);
                assert_eq!(*range.end().addr(), 0x2000);

                let pages: Vec<_> = range.iter().collect();
                assert_eq!(pages.len(), 1);
                assert_eq!(pages[0], page);
            }

            #[test]
            fn test_large_page_ranges() {
                // Test with 2MB pages
                let start_2m = $page_type::new_2m(<$addr_type>::new(0x200000)).unwrap();
                let range_2m = $page_range_type::new(start_2m, 2);

                assert_eq!(range_2m.len(), 2);
                assert_eq!(range_2m.addr_len(), 2 * 0x200000);
                assert_eq!(*range_2m.end().addr(), 0x600000);

                // Test with 1GB pages
                let start_1g = $page_type::new_1g(<$addr_type>::new(0x40000000)).unwrap();
                let range_1g = $page_range_type::new(start_1g, 2);

                assert_eq!(range_1g.len(), 2);
                assert_eq!(range_1g.addr_len(), 2 * 0x40000000);
                assert_eq!(*range_1g.end().addr(), 0xC0000000);
            }

            #[test]
            fn test_page_range_display() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range = $page_range_type::new(start_page, 3);

                let display_str = format!("{}", range);
                assert!(display_str.contains(stringify!($page_range_type)));
                assert!(display_str.contains("3 pages"));
                assert!(display_str.contains("0x1000"));
                assert!(display_str.contains("0x4000"));
            }

            #[test]
            fn test_page_range_debug() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range = $page_range_type::new(start_page, 2);

                let debug_str = format!("{:?}", range);
                assert!(debug_str.contains(stringify!($page_range_type)));
            }

            #[test]
            fn test_page_range_clone_copy() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range = $page_range_type::new(start_page, 2);

                // Test Clone
                let cloned = range.clone();
                assert_eq!(range.start(), cloned.start());
                assert_eq!(range.len(), cloned.len());

                // Test Copy
                let copied = range;
                assert_eq!(range.start(), copied.start());
                assert_eq!(range.len(), copied.len());
            }

            #[test]
            fn test_page_range_equality() {
                let start1 = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range1 = $page_range_type::new(start1, 3);

                let start2 = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range2 = $page_range_type::new(start2, 3);

                let start3 = $page_type::new_4k(<$addr_type>::new(0x2000)).unwrap();
                let range3 = $page_range_type::new(start3, 3);

                let range4 = $page_range_type::new(start1, 2);

                assert_eq!(range1, range2); // Same start and length
                assert_ne!(range1, range3); // Different start
                assert_ne!(range1, range4); // Different length
            }

            #[test]
            fn test_range_iterator_clone() {
                let start_page = $page_type::new_4k(<$addr_type>::new(0x1000)).unwrap();
                let range = $page_range_type::new(start_page, 3);

                let mut iter1 = range.iter();
                let mut iter2 = iter1.clone();

                // Both iterators should work independently
                assert_eq!(iter1.next().unwrap().addr(), iter2.next().unwrap().addr());

                let remaining1: Vec<_> = iter1.collect();
                let remaining2: Vec<_> = iter2.collect();

                assert_eq!(remaining1.len(), remaining2.len());
            }

            #[test]
            fn test_edge_cases() {
                // Test with maximum address values (within reason)
                let high_addr = 0xFFFF_0000usize;
                let start_page = $page_type::new_4k(<$addr_type>::new(high_addr)).unwrap();
                let range = $page_range_type::new(start_page, 1);

                assert_eq!(*range.start().addr(), high_addr);
                assert_eq!(*range.end().addr(), high_addr + 0x1000);

                // Test large ranges
                let large_range = $page_range_type::new(start_page, 1000);
                assert_eq!(large_range.len(), 1000);
                assert_eq!(large_range.addr_len(), 1000 * 0x1000);
            }
        }
    };
}
