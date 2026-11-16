"""Resolve generated Video test assets after copying a portable verification pack."""
from pathlib import Path


def fixture_path(profile, fixture, key):
    path = Path(fixture[key])
    if fixture.get("portable"):
        assert not path.is_absolute(), (key, path)
        path = (profile / path).resolve()
        assert path.is_relative_to(profile.resolve()), (key, path)
    return path.resolve()


def runtime_environment(profile, fixture):
    if not fixture.get("portable"):
        return {}
    # One fresh profile owns this whole temporary directory. Discovering its one
    # cache root avoids copying a user-specific TMPDIR or duplicating SipHash.
    temporary = profile.parent / (profile.name + "-runtime")
    temporary.mkdir(exist_ok=True)
    assert not temporary.is_symlink()
    return {"TMPDIR": str(temporary.resolve()) + "/"}


def fixture_cache(profile, fixture):
    if not fixture.get("portable"):
        return Path(fixture["cache"])
    temporary = profile.parent / (profile.name + "-runtime")
    base = temporary / "Vixen/VideoEffect/mac-v1"
    roots = [path for path in base.glob("*") if (path / ".owner").is_file()]
    assert len(roots) <= 1, f"Unexpected cache owners in private fixture: {roots}"
    return roots[0] if roots else base / "not-created-yet"
