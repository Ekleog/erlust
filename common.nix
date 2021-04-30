rec {
  hostPkgs = import <nixpkgs> {};
  pkgsSrc = hostPkgs.fetchFromGitHub {
    owner = "NixOS";
    repo = "nixpkgs";
    # The following is for nixos-unstable as of 2021-04-30
    rev = "8e4fe32876ca15e3d5eb3ecd3ca0b224417f5f17";
    sha256 = "1l7bnn2mlwmbi6s9kqa1g2i66arzshqijym1qmqq5417q5pq1da7";
  };
  rustOverlaySrc = hostPkgs.fetchFromGitHub {
    owner = "mozilla";
    repo = "nixpkgs-mozilla";
    # The following is the latest version as of 2021-04-30
    rev = "8c007b60731c07dd7a052cce508de3bb1ae849b4";
    sha256 = "1zybp62zz0h077zm2zmqs2wcg3whg6jqaah9hcl1gv4x8af4zhs6";
  };
  rustOverlay = import rustOverlaySrc;
  pkgs = import pkgsSrc {
    overlays = [ rustOverlay ];
  };
  rustNightlyChannel = pkgs.rustChannelOf {
    date = "2021-04-25";
    channel = "nightly";
    sha256 = "1azg3jcgaaqkrg585x8v9kk4p65b03sdmkhl3g8vmh0ckaizl82y";
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
