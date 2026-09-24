#!/usr/bin/env bash
set -u

source_dir="$(cd "$(dirname "$0")" && pwd)"
sandbox="$(mktemp -d)"
trap 'rm -rf "$sandbox"' EXIT

failures=0
repo=""

fail() {
    echo "FAIL: $1"
    failures=$((failures + 1))
}

assert_equals() {
    [ "$1" = "$2" ] || fail "$3: expected '$1', got '$2'"
}

new_repo() {
    repo="$(mktemp -d "$sandbox/repo.XXXXXX")"
    git -C "$repo" init -q -b main
    git -C "$repo" -c user.name=t -c user.email=t@t commit -q --allow-empty -m init
    mkdir -p "$repo/scripts" "$repo/docs/specs/archive"
    cp "$source_dir/new-spec" "$repo/scripts/new-spec"
}

new_spec() {
    (cd "$repo" && scripts/new-spec "$@")
}

spec_files() {
    find "$repo/docs" -name '*.md' | sort
}

test_empty_tree_starts_at_one() {
    new_repo
    path="$(echo body | new_spec first)"
    assert_equals "docs/specs/001-first.md" "$path" "empty tree path"
    assert_equals "body" "$(cat "$repo/$path")" "written body"
}

test_number_is_highest_across_specs_and_archive_plus_one() {
    new_repo
    touch "$repo/docs/specs/009-a.md" "$repo/docs/specs/archive/012-b.md"
    path="$(echo body | new_spec next)"
    assert_equals "docs/specs/013-next.md" "$path" "highest plus one"
}

test_duplicate_numbers_in_archive_are_harmless() {
    new_repo
    touch "$repo/docs/specs/archive/028-a.md" "$repo/docs/specs/archive/028-b.md"
    path="$(echo body | new_spec next)"
    assert_equals "docs/specs/029-next.md" "$path" "duplicates"
}

test_leading_zero_is_not_octal() {
    new_repo
    touch "$repo/docs/specs/008-a.md"
    path="$(echo body | new_spec next)"
    assert_equals "docs/specs/009-next.md" "$path" "008 plus one"
}

test_parallel_invocations_get_distinct_gapless_numbers() {
    new_repo
    count=20
    for i in $(seq "$count"); do
        (echo body | new_spec "slug-$i" >"$sandbox/out.$i") &
    done
    wait
    numbers="$(cat "$sandbox"/out.* | sed -E 's|.*/([0-9]{3})-.*|\1|' | sort)"
    expected="$(seq -f '%03g' "$count")"
    assert_equals "$expected" "$numbers" "parallel numbers"
    rm -f "$sandbox"/out.*
}

test_refuses_off_main_and_writes_nothing() {
    new_repo
    git -C "$repo" checkout -q -b feature
    echo body | new_spec x 2>"$sandbox/err"
    status=$?
    [ "$status" -ne 0 ] || fail "off main should exit non-zero"
    grep -q "specs are only added on main" "$sandbox/err" || fail "off main should explain itself"
    assert_equals "" "$(spec_files)" "off main files"
}

assert_rejected_without_files() {
    description="$1"
    shift
    status=0
    "$@" 2>/dev/null || status=$?
    [ "$status" -ne 0 ] || fail "$description should exit non-zero"
    assert_equals "" "$(spec_files)" "$description files"
}

test_rejects_missing_slug() {
    new_repo
    assert_rejected_without_files "missing slug" bash -c 'cd "$1" && echo body | scripts/new-spec' _ "$repo"
}

test_rejects_uppercase_slug() {
    new_repo
    assert_rejected_without_files "uppercase slug" bash -c 'cd "$1" && echo body | scripts/new-spec Bad' _ "$repo"
}

test_rejects_empty_stdin() {
    new_repo
    assert_rejected_without_files "empty stdin" bash -c 'cd "$1" && scripts/new-spec ok </dev/null' _ "$repo"
}

test_lock_is_released_after_failed_run() {
    new_repo
    git -C "$repo" checkout -q -b feature
    echo body | new_spec x 2>/dev/null
    git -C "$repo" checkout -q main
    echo body | new_spec x >/dev/null 2>&1 || fail "run after failed run should succeed"
    [ ! -e "$repo/.git/spec.lock" ] || fail "lock should be gone after success"
}

test_lock_is_released_after_write_failure() {
    new_repo
    chmod a-w "$repo/docs/specs"
    echo body | new_spec x 2>/dev/null
    chmod u+w "$repo/docs/specs"
    [ ! -e "$repo/.git/spec.lock" ] || fail "lock should be gone after write failure"
}

for test in $(declare -F | awk '$3 ~ /^test_/ {print $3}'); do
    "$test"
done

if [ "$failures" -ne 0 ]; then
    echo "$failures failure(s)"
    exit 1
fi
echo "all new-spec tests passed"
