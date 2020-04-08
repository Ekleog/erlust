rec {
  hostPkgs = import <nixpkgs> {};
  pkgsSrc = hostPkgs.fetchFromGitHub {
    owner = "NixOS";
    repo = "nixpkgs";
    # The following is for nixos-unstable as of 2020-04-04
    rev = "ae6bdcc53584aaf20211ce1814bea97ece08a248";
    sha256 = "0hjhznns1cxgl3hww2d5si6vhy36pnm53hms9h338v6r633dcy77";
  };
  rustOverlaySrc = hostPkgs.fetchFromGitHub {
    owner = "mozilla";
    repo = "nixpkgs-mozilla";
    # The following is the latest version as of 2020-04-04
    rev = "e912ed483e980dfb4666ae0ed17845c4220e5e7c";
    sha256 = "08fvzb8w80bkkabc1iyhzd15f4sm7ra10jn32kfch5klgl0gj3j3";
  };
  rustOverlay = import rustOverlaySrc;
  pkgs = import pkgsSrc {
    overlays = [ rustOverlay ];
  };
  rustNightlyChannel = pkgs.rustChannelOf {
    date = "2019-03-16";
    channel = "nightly";
    sha256 = "1ia8kpdd26wi23xarih2wb0ppv61irb1qqbh34xzl7zi15j83xwh";
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
