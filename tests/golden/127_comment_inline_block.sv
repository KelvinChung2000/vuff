module m;
  initial begin
    /* c */ x = 1;
    /* tag */ y = 2;
  end
endmodule
// expected -----
module m;
  initial begin
    /* c */ x = 1;
    /* tag */ y = 2;
  end
endmodule
