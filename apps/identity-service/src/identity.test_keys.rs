#[cfg(test)]
pub fn test_rsa_private_key_pem() -> String {
    format!(
        "-----BEGIN {} PRIVATE KEY-----\n{}\n-----END {} PRIVATE KEY-----",
        "",
        concat!(
            "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDMU9Csf4fUU/PD\n",
            "gODCUoCzzikKf7yk/p1TnxqXJCKWlwfB8+JGItss3haaMjcQ0tzRlJFJJ0hnPvvF\n",
            "BCnsLiy4nFJiR2GjryW6/wGpcItGnHPoSnaNO2BMSR+YlzNAS0xhVHc61bgFS9pb\n",
            "GfEapGYFs1P+Kq0pBf0yNs3bPGeylB/wRW19xb75TeEFBhnqvXwfbdiTjtEwfuu4\n",
            "fczveSKxBBLX70xVXRRZfD/DjfW0KrjreaScTKJ66ZRtWGtKl/a8gGVGpeHHgYVR\n",
            "4drd0ne+8ozC3y/Kn59glTm05zurKHspNL4gcQ7LxShC2z1xLnzPdF88UOUFtFuS\n",
            "HZ/B9dpFAgMBAAECggEAGEpSgV8714sPbItyrMdAE6ALkvrygy7dpyY/8/3QutRD\n",
            "kNQcbzgPlDrmtvgHQdG/fI7L4sVvvw5mwdon3bLzkBLNXG+d9OaKdruACBOgCwno\n",
            "YZIQm+OWJBuBTROUfS02bE+tSOpFUSPeVnw0BHXMxxq1fn62TS0wF3saZ9i7fygU\n",
            "ZWkXtEGjLxI07ctUwkHA7J9SkYm8QGyiS2RYZAyh0as8rhEGwaHqOVIZphqQ/AG9\n",
            "gb8aentM9gy4uStNUdW1xacUa9jniE0PxR+4Jt0qksmgLJc1G2qz+goKXHC8a4+H\n",
            "aZqpjkANTbfZbbt22AEqapsy2JBW25i6IPBNIrghFQKBgQDtYWPSOFNcidBvh1VX\n",
            "9tI8ORXZQrMl2+ncRrghNQBk1GIOWowxjzLww5NDKPWiMG2H8Cco7neJlH+Z6vs1\n",
            "EcBagGwSS59jEyujWX7QaeLKtEEzBrpOEI19YgxTWS4N2k5NAYF3g8weR/Jr0o3y\n",
            "ObMz9ABEuGVGUk6MKA2kw9lTlwKBgQDcWrgJbyIH/F/FrJqPXUdxxDqEXTxYfTL0\n",
            "TXFMKcMrmV6t3XEkFY+1iPQHNeB6/dmY1MrFOtXOO/RzkDJZLc0Ciz1+SlW8qy5q\n",
            "2jTucDZ4sybBQ8Vns/ckoaS3v86TCvsBueKNyT7+lf0myER4BSzOCjlFqo5Jw4t5\n",
            "mV1LO3MMgwKBgQCKbcLScrpaOpvsjhU8yNjs+bU+D2F9cHM+W5dA9jGWmyvbhv4+\n",
            "YG2qbcLQ5W/o9yjIn0mW2wmml4yZ66g22HU90ao0ORlno2RNTAFh9H2nC9sBsKiw\n",
            "oYKBXc4mRNlQhsAms/wWACvmdLpwGkdgvDk+0MnfSVD140WfAjSCoxt3XQKBgBfF\n",
            "eaEa6hLueO58Rlg8+d4eCyoIXOA28W5FhHlw7+seKoabIv9/i/dLhPfaKhNam0TP\n",
            "f+hzBmmvMhndbnEMbddeag3buxAVb3Z7f8ZROK8gtIeY5gzf70N2ZKyl9oUKZDW1\n",
            "delR0ofoalzqseg4trKri64mTh9LBxrhHp1lFm49AoGAaxHvdnqrBbN50WwThsCG\n",
            "0p5Xzl9D709WfxPQAL9KifdtpJhmKjk2PbLlLnZT7gaKbvjXWu3bv/c3WqLQgojw\n",
            "RtzxztqGplco0Z1Mdv7rZ1HrsPgBShkV4fnFxK5svRU7HSa66lVepzvKcOMrcGYH\n",
            "mysOqbOCznEnALlWRSK8n6I="
        ),
        ""
    )
}

#[cfg(test)]
pub const TEST_RSA_PUBLIC_KEY_PEM: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAzFPQrH+H1FPzw4DgwlKA
s84pCn+8pP6dU58alyQilpcHwfPiRiLbLN4WmjI3ENLc0ZSRSSdIZz77xQQp7C4s
uJxSYkdho68luv8BqXCLRpxz6Ep2jTtgTEkfmJczQEtMYVR3OtW4BUvaWxnxGqRm
BbNT/iqtKQX9MjbN2zxnspQf8EVtfcW++U3hBQYZ6r18H23Yk47RMH7ruH3M73ki
sQQS1+9MVV0UWXw/w431tCq463mknEyieumUbVhrSpf2vIBlRqXhx4GFUeHa3dJ3
vvKMwt8vyp+fYJU5tOc7qyh7KTS+IHEOy8UoQts9cS58z3RfPFDlBbRbkh2fwfXa
RQIDAQAB
-----END PUBLIC KEY-----"#;
