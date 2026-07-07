pub mod nvbes {
    pub mod platform {
        pub mod v1 {
            tonic::include_proto!("nvbes.platform.v1");
        }
    }

    pub mod enterprise {
        pub mod v1 {
            tonic::include_proto!("nvbes.enterprise.v1");
        }
    }
}
