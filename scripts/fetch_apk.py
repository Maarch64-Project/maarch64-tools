import urllib.request
import tarfile
import io
import os
import sys

BASE_URLS = [
    "https://dl-cdn.alpinelinux.org/alpine/latest-stable/main/aarch64/",
    "https://dl-cdn.alpinelinux.org/alpine/latest-stable/community/aarch64/"
]

TARGET_DIR = os.path.abspath("sysroot/aarch64-rootfs")

def parse_index(repo_url):
    index_url = repo_url + "APKINDEX.tar.gz"
    print(f"[*] Fetching index from {index_url}...")
    req = urllib.request.urlopen(index_url)
    packages = {}
    with tarfile.open(fileobj=io.BytesIO(req.read()), mode="r:gz") as tar:
        content = tar.extractfile("APKINDEX").read().decode("utf-8", errors="ignore")
        current_pkg = {}
        for line in content.splitlines():
            if not line.strip():
                if "P" in current_pkg:
                    packages[current_pkg["P"]] = current_pkg
                current_pkg = {}
            elif ":" in line:
                k, v = line.split(":", 1)
                current_pkg[k] = v
        if "P" in current_pkg:
            packages[current_pkg["P"]] = current_pkg
    return packages

def resolve_deps(pkg_name, all_pkgs, visited=None):
    if visited is None:
        visited = set()
    if pkg_name in visited:
        return []
    visited.add(pkg_name)

    if pkg_name not in all_pkgs:
        # Search for virtual or provided package
        found = None
        for p, data in all_pkgs.items():
            provides = data.get("p", "").split()
            for prov in provides:
                if prov.split("=")[0] == pkg_name:
                    found = p
                    break
            if found:
                break
        if found:
            pkg_name = found
        else:
            print(f"[!] Warning: Package {pkg_name} not found in index")
            return []

    pkg_data = all_pkgs[pkg_name]
    deps = [pkg_name]
    raw_deps = pkg_data.get("D", "").split()
    for d in raw_deps:
        # Strip versions/operators like so:name or dep>=1.0
        clean_dep = d.split("=")[0].split(">")[0].split("<")[0].split("~")[0]
        if clean_dep.startswith("so:"):
            # Shared lib dependency, find providing package
            so_name = clean_dep[3:]
            prov_pkg = None
            for p, data in all_pkgs.items():
                provides = data.get("p", "").split()
                if any(pr.startswith(f"so:{so_name}") for pr in provides):
                    prov_pkg = p
                    break
            if prov_pkg:
                deps.extend(resolve_deps(prov_pkg, all_pkgs, visited))
        elif clean_dep.startswith("cmd:"):
            continue
        else:
            deps.extend(resolve_deps(clean_dep, all_pkgs, visited))
    return deps

def main():
    target_pkgs = sys.argv[1:] if len(sys.argv) > 1 else ["vlc", "vlc-libs"]
    print(f"[*] Target packages: {target_pkgs}")

    all_pkgs = {}
    pkg_repo_map = {}

    for repo in BASE_URLS:
        pkgs = parse_index(repo)
        for name, data in pkgs.items():
            all_pkgs[name] = data
            pkg_repo_map[name] = repo

    to_install = []
    visited = set()
    for target in target_pkgs:
        deps = resolve_deps(target, all_pkgs, visited)
        for d in deps:
            if d not in to_install:
                to_install.append(d)

    print(f"[+] Total packages to download ({len(to_install)}): {to_install}")

    for pkg_name in to_install:
        if pkg_name not in all_pkgs:
            continue
        data = all_pkgs[pkg_name]
        filename = f"{data['P']}-{data['V']}.apk"
        repo_url = pkg_repo_map[pkg_name]
        apk_url = repo_url + filename

        print(f"[*] Downloading {filename}...")
        try:
            req = urllib.request.urlopen(apk_url)
            apk_bytes = req.read()
            # Extract tar.gz inside apk
            with tarfile.open(fileobj=io.BytesIO(apk_bytes), mode="r:*") as tar:
                tar.extractall(path=TARGET_DIR)
            print(f"  [+] Extracted {filename} to {TARGET_DIR}")
        except Exception as e:
            print(f"  [!] Failed to download/extract {filename}: {e}")

    print("[+] All packages installed successfully into AArch64 rootfs!")

if __name__ == "__main__":
    main()
