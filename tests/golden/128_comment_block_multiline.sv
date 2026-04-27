module m;
  /* block
     comment */
  wire x;
  /*
    multi
    line
    block
  */
  reg y;
endmodule
// expected -----
module m;
  /* block
     comment */
  wire x;
  /*
    multi
    line
    block
  */
  reg y;
endmodule
