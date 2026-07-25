#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
java_repo="${SA_TOKEN_JAVA_REPO:-${repo_root}/../Sa-Token}"
expected_commit="902886c2149261ccb53a9c982068b7ccd0990237"
actual_commit="$(git -C "${java_repo}" rev-parse HEAD)"

if [[ "${actual_commit}" != "${expected_commit}" ]]; then
    echo "Java checkout mismatch: expected ${expected_commit}, got ${actual_commit}" >&2
    echo "Review the new Java behavior before changing the pinned golden fixture." >&2
    exit 1
fi

tmp_dir="$(mktemp -d)"
cleanup() {
    rm -rf "${tmp_dir}"
}
trap cleanup EXIT

mvn -q -f "${java_repo}/pom.xml" -pl sa-token-core -am -DskipTests package
mvn -q -f "${java_repo}/sa-token-core/pom.xml" \
    dependency:build-classpath \
    "-Dmdep.outputFile=${tmp_dir}/classpath.txt"

core_jar="${java_repo}/sa-token-core/target/sa-token-core-1.45.0.jar"
classpath="${core_jar}:$(<"${tmp_dir}/classpath.txt")"
javac -encoding UTF-8 \
    -cp "${classpath}" \
    -d "${tmp_dir}" \
    "${repo_root}/tests/java_golden/SaTokenGoldenExporter.java"
java -cp "${tmp_dir}:${classpath}" SaTokenGoldenExporter "${actual_commit}" \
    > "${repo_root}/tests/java_golden/sa_token_1_45_0.json"

echo "Updated tests/java_golden/sa_token_1_45_0.json from ${actual_commit}"
