//! Future live-token graph adapter.
//!
//! The first implementation builds analysis jobs from indexed historical blocks.
//! This module is reserved for a later path that starts from an already-live
//! `BlockTokenProcessor` graph and only loads the second-order context blocks.
