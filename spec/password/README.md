# Password-Format Source Lock

This directory retains the exact source material used by
`base64-ng-password`:

- `passlib-1.7.4-pbkdf2.rst` is the Passlib 1.7.4 PBKDF2 format page source;
- `SHA-crypt.txt` is Ulrich Drepper's SHA-crypt specification;
- `ERRATA.tsv` records the reviewed project snapshot; and
- `requirements.json` maps the claimed field and record transforms to tests.

These files are repository evidence and are excluded from published crates.
Run `scripts/validate-password-spec.py` to verify the lock offline.
