`ifdef DEBUG
  `define debug(cmd) cmd
`else
  `define debug(cmd)
`endif
module m;
  initial begin
    `debug($display("hello"))
    x <= 1;
  end
endmodule
// expected -----
`ifdef DEBUG
  `define debug(cmd) cmd
`else
  `define debug(cmd)
`endif
module m;
  initial begin
    `debug($display("hello"))
    x <= 1;
  end
endmodule
