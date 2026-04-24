// config: wrap_default_nettype=true
module a;
endmodule

module b;
endmodule
// expected -----
`default_nettype none
module a;
endmodule
`default_nettype wire

`default_nettype none
module b;
endmodule
`default_nettype wire
