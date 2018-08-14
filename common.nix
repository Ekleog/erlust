rec {
  hostPkgs = import <nixpkgs> {};
  pkgsSrc = hostPkgs.fetchFromGitHub {
    owner = "NixOS";
    repo = "nixpkgs";
    # The following is for nixos-unstable on 2018-08-09
    rev = "2428f5dda13475afba2dee93f4beb2bd97086930";
    sha256 = "1iwl5yaz36lf7v4hps3z9dl3zyq363jmr5m7y4anf0lpn4lczh18";
  };
  rustOverlaySrc = hostPkgs.fetchFromGitHub {
    owner = "mozilla";
    repo = "nixpkgs-mozilla";
    # The following is the latest version as of 2018-08-09
    rev = "18186b9786bc72b7de124c3be6eb1e69d1c9acca";
    sha256 = "09p0xa8syv0vjhpy0p4safcixy637zg7gln5gfw02jxaxb7nsbip";
  };
  rustOverlay = import rustOverlaySrc;
  pkgs = import pkgsSrc {
    overlays = [ rustOverlay ];
  };
  rustNightlyChannel = pkgs.rustChannelOf {
    date = "2018-08-14";
    channel = "nightly";
  };
  #rustBetaChannel = pkgs.rustChannelOf {
  #  date = "2018-04-20";
  #  channel = "beta";
  #};
  #rustStableChannel = pkgs.rustChannelOf {
  #  date = "2018-06-05";
  #  channel = "stable";
  #};
}
