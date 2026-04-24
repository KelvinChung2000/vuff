// config: wrap_default_nettype=true
module m;
  assign a = 1;
endmodule
// expected -----
`default_nettype none
module m;
  assign a = 1;
endmodule
`default_nettype wire
