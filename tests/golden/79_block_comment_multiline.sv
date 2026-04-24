module m;
  /* Multi-line comment
     that spans lines
     preserves indent
  */
  assign c = 3;
endmodule
// expected -----
module m;
  /* Multi-line comment
     that spans lines
     preserves indent
  */
  assign c = 3;
endmodule
