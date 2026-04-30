module m;
`ifdef A
  logic a;
`elsif B
  logic b;
`else
  logic c;
`endif
endmodule
// expected -----
module m;
  `ifdef A
    logic a;
  `elsif B
    logic b;
  `else
    logic c;
  `endif
endmodule
