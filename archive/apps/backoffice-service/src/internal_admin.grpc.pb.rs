pub mod nvbes {
    pub mod platform {
        pub mod v1 {
            tonic::include_proto!("nvbes.platform.v1");
        }
    }

    pub mod billing {
        pub mod v1 {
            tonic::include_proto!("nvbes.billing.v1");
        }
    }

    pub mod identity {
        pub mod internal {
            pub mod v1 {
                tonic::include_proto!("nvbes.identity.internal.v1");
            }
        }
    }
}
