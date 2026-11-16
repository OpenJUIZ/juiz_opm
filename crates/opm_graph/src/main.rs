// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! `opm_graph`: the same as `juiz_opm draw` (see [`opm_graph::cli`]).

fn main() -> std::process::ExitCode {
    opm_graph::cli::run("opm_graph", std::env::args().skip(1).collect())
}
