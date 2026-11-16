// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! `opm_opl`: the same as `juiz_opm opl` (see [`opm_opl::cli`]).

fn main() -> std::process::ExitCode {
    opm_opl::cli::run("opm_opl", std::env::args().skip(1).collect())
}
