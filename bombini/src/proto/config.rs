// This file is @generated by prost-build.
/// Configuration file for ProcMon detector
#[derive(serde::Deserialize, Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct ProcMonConfig {
    /// Flag for exporting events from kernel to user mode.
    #[prost(bool, tag = "1")]
    pub expose_events: bool,
    /// Process Filter Configuration.
    #[prost(message, optional, tag = "2")]
    pub process_filter: ::core::option::Option<ProcessFilter>,
}
/// Filter Events using process information.
/// Filtering is based on pattern: uid AND euid AND auid AND (binary.name OR binary.prefix OR binary.path).
/// All variables in the pattern are optional. if deny_list is true filter acts as a deny list, otherwise it
/// is an allow list.
#[derive(serde::Deserialize)]
#[serde(default)]
#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct ProcessFilter {
    /// List of UID's to filter.
    #[prost(uint32, repeated, tag = "1")]
    pub uid: ::prost::alloc::vec::Vec<u32>,
    /// List of EUID's to filter.
    #[prost(uint32, repeated, tag = "2")]
    pub euid: ::prost::alloc::vec::Vec<u32>,
    /// List of AUID's (login uid) to filter.
    #[prost(uint32, repeated, tag = "3")]
    pub auid: ::prost::alloc::vec::Vec<u32>,
    /// Binary filter args
    #[prost(message, optional, tag = "4")]
    pub binary: ::core::option::Option<BinaryFilter>,
    /// if true acts like deny list
    #[prost(bool, tag = "5")]
    pub deny_list: bool,
}
/// Binary filtering args
#[derive(serde::Deserialize)]
#[serde(default)]
#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct BinaryFilter {
    /// List of executables names to filter.
    #[prost(string, repeated, tag = "1")]
    pub name: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// List of full executable paths to filter.
    #[prost(string, repeated, tag = "2")]
    pub path: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// List of executable path prefixes to filter.
    #[prost(string, repeated, tag = "3")]
    pub prefix: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
/// Configuration file for FileMon detector.
#[derive(serde::Deserialize, Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct FileMonConfig {
    /// security_file_open config.
    #[prost(message, optional, tag = "1")]
    pub file_open: ::core::option::Option<FileHookConfig>,
    /// security_path_truncate config.
    #[prost(message, optional, tag = "2")]
    pub path_truncate: ::core::option::Option<FileHookConfig>,
    /// security_path_unlink config.
    #[prost(message, optional, tag = "3")]
    pub path_unlink: ::core::option::Option<FileHookConfig>,
    /// Filter File events by Process information.
    #[prost(message, optional, tag = "4")]
    pub process_filter: ::core::option::Option<ProcessFilter>,
}
/// FileMon hook configuration
#[derive(serde::Deserialize, Clone, Copy, PartialEq, Eq, Hash, ::prost::Message)]
pub struct FileHookConfig {
    /// do not load ebpf program
    #[prost(bool, tag = "1")]
    pub disable: bool,
}
/// Configuration file for NetMon detector.
#[derive(serde::Deserialize, Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct NetMonConfig {
    /// Filter Network events by Process information.
    #[prost(message, optional, tag = "1")]
    pub process_filter: ::core::option::Option<ProcessFilter>,
}
/// Configuration file for IOUringMon detector.
#[derive(serde::Deserialize, Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct IoUringMonConfig {
    /// Filter io_uring events by Process information.
    #[prost(message, optional, tag = "1")]
    pub process_filter: ::core::option::Option<ProcessFilter>,
}
/// Configuration file for GTFOBinsDetector.
#[derive(serde::Deserialize, Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct GtfoBinsConfig {
    /// Block execution of GTFOBins binaries.
    #[prost(bool, tag = "1")]
    pub enforce: bool,
    /// GTFOBins executables names.
    #[prost(string, repeated, tag = "2")]
    pub gtfobins: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
