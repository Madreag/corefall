//! cf-server library surface.
//!
//! The cf-server binary is a thin wrapper around the modes M0 stubbed +
//! M36 / M8B / M40 etc. progressively fill in. Production wiring of the
//! NAT punch flow + protocol semver gate lives in [`m8b_nat_punch`].

pub mod m8b_nat_punch;
