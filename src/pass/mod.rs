// Copyright (c) 2017-2021 Fabian Schuiki

//! Optimization and analysis passes on LLHD IR.
//!
//! This module implements various passes that analyze or mutate an LLHD
//! intermediate representation.
#![allow(
    clippy::collapsible_if,
    clippy::get_first,
    clippy::iter_kv_map,
    clippy::needless_borrow,
    clippy::only_used_in_recursion,
    clippy::question_mark,
    clippy::single_match
)]

pub mod cf;
pub mod cfs;
pub mod dce;
pub mod deseq;
pub mod ecm;
pub mod gcse;
pub mod insim;
pub mod proclower;
pub mod tcm;
pub mod vtpp;

pub use cf::ConstFolding;
pub use cfs::ControlFlowSimplification;
pub use dce::DeadCodeElim;
pub use deseq::Desequentialization;
pub use ecm::EarlyCodeMotion;
pub use gcse::GlobalCommonSubexprElim;
pub use insim::InstSimplification;
pub use proclower::ProcessLowering;
pub use tcm::TemporalCodeMotion;
pub use vtpp::VarToPhiPromotion;
