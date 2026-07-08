import React from "react";
import ccLogo from "../../assets/cc-logo-white.png";

// Car-Controlling wordmark (white, for the dark corporate UI). Kept under the
// original file name so all existing import sites (sidebar, onboarding) resolve
// unchanged.
const HandyTextLogo = ({
  width,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  return (
    <img
      src={ccLogo}
      alt="Car Controlling"
      width={width}
      className={className}
      style={{ height: "auto", display: "block" }}
    />
  );
};

export default HandyTextLogo;
