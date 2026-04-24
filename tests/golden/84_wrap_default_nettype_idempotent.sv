// config: wrap_default_nettype=true
`default_nettype none
module m;
  assign a = 1;
endmodule
`default_nettype wire
// expected -----
`default_nettype none
module m;
  assign a = 1;
endmodule
`default_nettype wire
