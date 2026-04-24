module m;
  assign y = sel ? arr[7:4] : arr[3:0];
endmodule
// expected -----
module m;
  assign y = sel ? arr[7:4] : arr[3:0];
endmodule
