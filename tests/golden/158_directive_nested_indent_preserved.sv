module m;
`ifdef A
  `ifdef B
    `define X 1
  `else
    `define X 0
  `endif
`endif
endmodule
// expected -----
module m;
`ifdef A
  `ifdef B
    `define X 1
  `else
    `define X 0
  `endif
`endif
endmodule
