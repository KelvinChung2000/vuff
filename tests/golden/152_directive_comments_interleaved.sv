// `default_nettype none
// `define DEBUGNETS
// uncomment for extras
`ifdef DEBUG
`define DEBUG_BODY 1
`else
`define DEBUG_BODY 0
`endif
// after the conditional
`define VERSION 1
module m;
endmodule
// expected -----
// `default_nettype none
// `define DEBUGNETS
// uncomment for extras
`ifdef DEBUG
`else
`define DEBUG_BODY 0
`endif
// after the conditional
`define VERSION 1
module m;
endmodule
