//! End-to-end behaviour of the chunking pipeline over a real-document and generated corpus.
//!
//! Covers equivalence with the pre-optimization output (golden digests), structural correctness,
//! optimality of the greedy packing, and scaling.
#![cfg(feature = "chunking")]

mod corpus;
mod defects;
mod differential;
mod digest;
mod fixtures;
mod golden;
mod optimality;
mod oracle;
mod properties;
mod scaling;
