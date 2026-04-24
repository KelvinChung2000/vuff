module m;
`ifdef A
`ifdef B
  logic a;
`endif
`endif
endmodule
// expected -----
module m;
`ifdef A
`ifdef B
`endif
`endif
endmodule
