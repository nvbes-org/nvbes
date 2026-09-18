pub mod nvbes {
    pub mod billing {
        pub mod v1 {
            tonic::include_proto!("nvbes.billing.v1");
        }
    }

    pub mod platform {
        pub mod v1 {
            tonic::include_proto!("nvbes.platform.v1");
        }
    }
}
