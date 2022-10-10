# Nix Template

A minimal template engine for deterministic nix configurations.

All files with `.tmpl.nix` suffix are processed by the template engine.
Use minijinja format to write your templates.

## Get Started

```shell
# Use binary cache
$ nix run nixpkgs#cachix use lightquantum
# Instantiate templates
$ nix run github:PhotonQuantum/nix-template
# Update lock file
$ nix run github:PhotonQuantum/nix-template update
```

This package is also available in [my NUR repository](https://github.com/PhotonQuantum/nur-packages)

## Why should I use this?

Consider the following example:

```nix
program.zsh = {
    enable = true;
    plugins = [
        {
          name = "input";
          file = "init.zsh";
          src = pkgs.fetchFromGitHub {
            owner = "zimfw";
            repo = "input";
            rev = "master"; # but I want a specific commit!
            # ... and I need to update sha256 manually
            sha256 = "sha256-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx="; 
          };
        }
    ];
}
```

This configuration is not deterministic. If upstream pushes a new commit, the build changes, and you need to update the
sha256 hash manually.

With nix-template, you can write the following:

```nix
program.zsh = {
    enable = true;
    plugins = [
        {
          name = "input";
          file = "init.zsh";
          src = pkgs.fetchFromGitHub {
            owner = "zimfw";
            repo = "input";
            # track master, but only when lock file is updated
            rev = "{{ commit_of_github('zimfw', 'input', 'master') }}";
            # no need to update sha256 manually
            sha256 = "{{ hash_from_github('zimfw', 'input', 'master') }}";
          };
        }
    ];
}
```

Now the non-deterministic part is isolated in the lock file. You can update the lock file by
running `nix-template update`.