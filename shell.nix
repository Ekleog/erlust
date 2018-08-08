with import ./common.nix;

pkgs.stdenv.mkDerivation {
  name = "erlust";
  buildInputs = (with rustNightlyChannel; [ rustfmt-preview ]) ++
                (with rustStableChannel; [ rust ]);
}
