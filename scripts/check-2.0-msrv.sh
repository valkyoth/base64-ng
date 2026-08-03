#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_MSRV_TOOLCHAIN:-1.90.0}"
mode="${1:---host}"

case "$mode" in
    --host)
        for features in \
            none \
            alloc \
            std \
            stream \
            simd \
            secrets \
            checked-backend \
            secrets,simd \
            all
        do
            echo "2.0 MSRV: host feature set $features"
            case "$features" in
                none)
                    cargo +"$toolchain" check --no-default-features --lib
                    ;;
                all)
                    cargo +"$toolchain" check --all-features --lib
                    ;;
                *)
                    cargo +"$toolchain" check --no-default-features \
                        --features "$features" --lib
                    ;;
            esac
        done

        for package in \
            base64-ng-derive \
            base64-ng-imap \
            base64-ng-mime \
            base64-ng-multibase \
            base64-ng-password \
            base64-ng-openpgp \
            base64-ng-pem \
            base64-ng-sanitization \
            base64-ng-serde \
            base64-ng-bytes \
            base64-ng-subtle \
            base64-ng-tokio
        do
            echo "2.0 MSRV: companion $package"
            cargo +"$toolchain" check -p "$package" --all-features
        done

        cargo +"$toolchain" check --locked \
            --manifest-path semantic-corpus/runner/Cargo.toml
        BASE64_NG_ALPHABET_TOOLCHAIN="$toolchain" \
            scripts/check-2.0-alphabet.sh
        ;;
    --target)
        target="${2:?usage: check-2.0-msrv.sh --target TARGET}"
        features="simd,secrets"
        case "$target" in
            wasm32-unknown-unknown)
                features="$features,allow-wasm32-best-effort-wipe"
                ;;
            s390x-unknown-linux-gnu|powerpc64-unknown-linux-gnu)
                features="$features,allow-compiler-fence-only-wipe"
                ;;
        esac
        echo "2.0 MSRV: $target no-default features $features"
        cargo +"$toolchain" check --target "$target" --no-default-features \
            --features "$features" --lib
        ;;
    *)
        echo "usage: $0 [--host|--target TARGET]" >&2
        exit 2
        ;;
esac

echo "2.0 MSRV: Rust $toolchain $mode checks ok"
