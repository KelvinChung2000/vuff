module m;
  logic a;
`ifdef DEBUG
  logic b;
`else
  logic c;
`endif
  logic d;
endmodule
// expected -----
module m;
  logic a;
  `ifdef DEBUG
    logic b;
  `else
    logic c;
  `endif
  logic d;
endmodule
