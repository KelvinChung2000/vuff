/* leading block */
module m;
/* inline block */
  assign a = 1;
endmodule
// expected -----
/* leading block */
module m;
  /* inline block */
  assign a = 1;
endmodule
